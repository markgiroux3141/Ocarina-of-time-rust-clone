# GAME-13: The Forest Temple

**Goal:** Phase 14 of the [roadmap](ROADMAP.md): adult Link gets the hookshot from Dampé's race,
crosses the Lost Woods past Mido and the meadow's Moblins, learns the Minuet of Forest from Sheik,
and clears the Forest Temple: the bow, the Poe sisters, the twisted corridors, Phantom Ganon. The
blue warp gives the Forest Medallion (Saria in the Chamber), and Kokiri Forest's Deku Sprout follows.
Exit (ROADMAP's outline): the Forest Medallion.

**Status:** surveyed, proposed and decided (2026-10-27: the user took the breakdown and every
recommendation: the calls' section). The phase starts from the `epona-won` preset (Phase 13's exit
review).

| # | Milestone | Status |
|---|---|---|
| 1 | The hookshot and Dampé's race | done (2026-10-27) |
| 2 | The Lost Woods, the meadow and the Minuet | |
| 3 | The temple's front and the bow | |
| 4 | The courtyards and the twisted corridors | |
| 5 | The Poe sisters | |
| 6 | Phantom Ganon and the Forest Medallion (the exit) | |

## The survey

Checked against the decomp (52a510f) and the pack (`out/data28`). `ootx scene-info` was run on every
layer of `Bmori1` and `moribossroom` (one setup each), `kenjyanoma` (0 to 6), and the adult layers
of `spot04`, `link_home`, the Kokiri houses and shop, `spot10`, `spot05`, `spot02` and
`hakasitarelay`. Two read-only helpers read the scenes and the C; their key claims were checked
against it: the Kokiri Forest layer rule, the Minuet's trigger, Mido's flag, Dampé's 60 seconds, the
bow's and the hookshot's chests, `Bg_Mori_Bigst`'s flag, `Door_Warp1`'s Forest branch, Phantom
Ganon's texture decay, and the sizes.

### The start: `epona-won`

Phase 13's exit review read it: adult, the Master Sword, the Hylian Shield, the Kokiri Tunic; sticks,
bombs, the slingshot, the Ocarina of Time, the boomerang, an empty bottle; five songs (Saria's among
them); six hearts; 50 rupees (a debug start's); Epona his. **No hookshot, no bow.** Link is in
Hyrule Field by day.

### The scenes and their layers

| Scene | Layer | Who's there (unported in **bold**) | Notes |
|---|---|---|---|
| `spot04` (Kokiri Forest) | 2 (an adult before the temple) | 10 `En_Dekubaba`, 2 `En_Okuta`, 3 `En_Dekunuts`, `En_Sw`, 3 `En_Gs`, `Obj_Bean` 0x0009, **`Obj_Mure3`** (a rupee ring by the bean's lift) | `Play_Init` picks 3 for an adult once `EVENTCHKINF_48` is set, else 2, whatever the time (`z_play.c:434-436`; ported). No Kokiri, no Mido for an adult before the medallion |
| | 3 (after the temple) | 7 `En_Ko` back, **`Obj_Dekujr`** (the Deku Sprout: killed without the Forest Medallion) | |
| | 12, 13 | the Sprout's cutscenes (`gKokiriForestDekuSproutPart1Cs`, `Part3Cs`) | after the Chamber (`CS_DEST_KOKIRI_FOREST_FROM_CHAMBER_OF_SAGES`) |
| Kokiri houses, Link's house, the shop | 2, 3 | the Know-It-All brothers' 5 `En_Ko`, `En_Md` and `En_Sa` killed for an adult | all ported; `en_ko.rs` has the adult texts |
| `spot10` (Lost Woods) | 2, 3 | **adult Mido** (`En_Md`, room 3: blocks the meadow until Saria's Song, `EVENTCHKINF_0A`; ported), 3 `En_Skj`, Fado, **`En_Hs`** (Grog, room 9), **`En_Mm2`** (the running man, killed unless the marathon's on) | the `epona-won` chain never set 0x0A: Mido blocks, as the C |
| `spot05` (Sacred Forest Meadow) | 2, 3 | **6 `En_Mb`** (5 spear Moblins on paths, 1 club Moblin), `En_Xc` 6 (Sheik: a placeholder), `Bg_Mjin`, `Door_Warp1` 6 | `En_Xc` 6 starts `gMeadowMinuetCs` once Link's z < -2225 (`z_en_xc.c:325-332`): `EVENTCHKINF_50`, the Minuet. Exit 2: `ENTR_FOREST_TEMPLE_0` (the ledge, by the hookshot) |
| `spot02` (graveyard) | 2, 3 | 3 `En_Poh`, `Bg_Spot02_Objects` (the grave's stone gone for an adult) | all ported |
| `hakasitarelay` (Dampé's grave) | 2, 3 | **`En_Po_Relay`** (Dampé's ghost), 4 `En_Rd`, the doors `Bg_Relay_Objects`, room 4's **hookshot chest** (`En_Box` 0x1100: `GI_HOOKSHOT`, room-clear) | the race: Dampé opens doors (switches 0x35-0x37) and drops fire (**`En_Honotrap`** flame drop). The first race `Flags_SetTempClear(4)` brings out the chest; later ones, 60 seconds or less, the heart piece (`z_en_po_relay.c:366-384`) |
| `Bmori1` (the Forest Temple, MQ) | one setup | 23 rooms: the table below | |
| `moribossroom` | one setup | **`Boss_Ganondrof`** | exit to `ENTR_FOREST_TEMPLE_1` (room 17) |
| `kenjyanoma` (the Chamber) | 0 to 3 | `Demo_Sa` 0 (Saria's Forest Medallion: a log only, BACKLOG #110) | |

**The Forest Temple's rooms** (MQ; the dungeon helper's table, chests decoded with `ENBOX_GET_*`):

| Room | Unported | Key contents |
|---|---|---|
| 0 entrance | | `En_Sw`, `En_St`, a switch dropping a small key |
| 1 hall | | 3 `En_St`; locked door to the lobby |
| 2 lobby | `Bg_Po_Syokudai` ×4 (the sisters' torches), `Bg_Mori_Elevator` | eye switches to the courtyards; the hole to the basement |
| 3 east hall, 4 north hall | | `En_Light`; a Song of Time block |
| 5 west hall | `En_Test` (Stalfos) | `Obj_Roomtimer`: flag 0x13 when cleared |
| 6 the bow's room | `Bg_Mori_Bigst` | 2 `En_Wf` (a key); up top 3 Stalfos and the falling ceiling, then flag 0x15: **the bow's chest** (0xB08C, `GI_BOW`) |
| 7, 8 courtyards | `Bg_Mori_Idomizu` (the well's water) | Deku Babas, Octoroks, hookshot targets, 3 keys, a web |
| 9 well, 10 bridge | | a key; the big `En_St` |
| 11 west block room | 2 `Obj_Blockstop`, 2 `En_Bb` (Blue Bubbles), `Bg_Mori_Hineri` | `Obj_Oshihiki` blocks, crystal switches (one toggles 0x13) |
| 12, 13 Joelle's and Beth's rooms | `Bg_Po_Event` (the paintings) | the map, the compass |
| 14 east upper room | `Bg_Mori_Hashira4` (rotating pillars), `Bg_Mori_Hineri` | a frozen eye switch (**`Obj_Ice_Poly`**, a lit arrow) |
| 15 falling ceiling | `Bg_Mori_Rakkatenjo` | 7 `En_St`, arrows |
| 16 Amy's room | `Bg_Po_Event` (her timed block puzzle) | |
| 17 basement | `Bg_Mori_Kaitenkabe` (the rotating wall), `Bg_Mori_Hashira4` gates ×3 | the boss door (flag 0x14) |
| 18 | | `En_Rd`, a key |
| 19, 20 twisted corridors | `En_Wallmas`, 2 `En_Bb` (Green), `Bg_Mori_Hineri` | 19 spawns **the boss key** once straightened |
| 21 | `En_Floormas` | |
| 22 boss antechamber | | to `moribossroom` |

Six small keys for six locked doors, five Gold Skulltulas. Spawned, not placed: **`En_Po_Sisters`**
(Meg, Joelle, Beth, Amy), **`En_fHG`** (Phantom Ganon's horse and intro), **`En_Fhg_Fire`** (his
lightning and energy balls), **`Arms_Hook`** (the hookshot), **`En_Honotrap`** (Dampé's fire),
**`Obj_Ice_Poly`**.

**Coverage:** COVERAGE's Phase 14 row is right (22 placed, 1 ported). Proposed correction: `En_Hs`
(Grog: the adult trade) and `En_Mm2` (the marathon) to Phase 20 (the user's call 2).

### The route

1. **To Dampé's grave** (Sheik at the pedestal says "Head to Kakariko Village!", 0x700F): the field
   to Kakariko (`ENTR_KAKARIKO_VILLAGE_0`), the graveyard (`ENTR_GRAVEYARD_0`), Dampé's grave
   (`ENTR_WINDMILL_AND_DAMPES_GRAVE_0`: the race's start beside Dampé).
2. **The race**, rooms 0 to 4, behind Dampé, his fire and the closing doors: the hookshot's chest.
   Out by the Song of Time blocks and the windmill to Kakariko (`ENTR_KAKARIKO_VILLAGE_8`).
3. **To the meadow:** the field to the Lost Woods' bridge (`ENTR_LOST_WOODS_8`), Kokiri Forest's
   layer 2 (Deku Babas), the Lost Woods (`ENTR_LOST_WOODS_0`): Mido in room 3, Saria's Song, the
   meadow (`ENTR_SACRED_FOREST_MEADOW_0`).
4. **The meadow's maze** past the Moblins' patrols to the top: Sheik and the Minuet; the hookshot to
   the ledge; the temple (`ENTR_FOREST_TEMPLE_0`).
5. **The temple** (the dungeon helper's order; parts unsure, to settle at their milestones): the
   front and the lobby (the sisters steal the flames); north to the bow; with the bow the eye
   switches, the courtyards and the paintings (Joelle, Beth); west to the straightened corridor and
   the boss key; east to Amy's blocks and the frozen eye; Meg; the elevator down; the rotating wall,
   the gates; the boss door.
6. **Phantom Ganon**, the blue warp: `EVENTCHKINF_48`, the Forest Medallion, the Chamber
   (`CHAMBER_CS_FOREST`: Saria), then Kokiri Forest's Sprout cutscenes (`CS_INDEX_8`) and its layer 3.

### What's already ported and reused

- **Player:** the bow is mostly there through the slingshot's code (`Player_InitBowOrSlingshotIA`,
  the arrow types with `ARROW_NORMAL_HORSE`, `CAM_MODE_AIM_ADULT`, the bowstring, the ammo on the
  C buttons); `En_Arrow` whole (not the magic arrows, BACKLOG #85). The hookshot's shared upper
  actions and animations are in (`player.rs:8033-8200`), and `Obj_Hsblock` is whole.
- **Grabs and hazards:** every Player cutscene action, `Play_TriggerRespawn`, `grabPlayer` (the
  ReDead's), crushing, the electrified knockback, `EffectSsFhgFlash`.
- **Dungeon systems:** small keys and locked doors, the boss door (the Forest texture),
  `Obj_Oshihiki`, `Obj_Timeblock`, `Obj_Switch`, the room timer, `Obj_Syokudai` (lit arrows), the HUD
  timers, `Door_Shutter`, `En_Box`'s room-clear and switch chests, the boss title cards' table
  (Phantom Ganon's is a row to add).
- **The ranch's and earlier enemies:** `En_St`, `En_Sw`, `En_Wf`, `En_Rd`, `En_Dekubaba`, `En_Okuta`,
  `En_Dekunuts`, `En_Poh`; Mido's ocarina check; `Door_Warp1`'s child warp and its sages' setup;
  `Demo_Effect`; the Prelude's warp (the Minuet's goes to `ENTR_SACRED_FOREST_MEADOW_2`, already in
  `sWarpSongEntrances`).

### The systems needed

| System | C | The port has | Missing |
|---|---|---|---|
| **The hookshot** | `ovl_Arms_Hook` (376); Player's spawn, `Player_UpperAction` fly start, `Player_Action_HookshotFly`, `Player_DestroyHookshot`, the reticle and its draw; `Camera_Special5`; `Actor_SwapHookshotAttachment`; `SurfaceType_CanHookshot` | the shared upper actions, the targets' flags, `Obj_Hsblock` | about 620 |
| **The bow** | Player's (shared with the slingshot), `En_Arrow` | nearly all | tests (adult, first person, from horseback) |
| **Dampé's race** | `En_Po_Relay` (458), `En_Honotrap`'s flame drop (about 250 of 559) | the doors, timers, `HS_DAMPE_RACE`, the room-clear chest | about 700 |
| **The meadow** | `En_Mb` (1,630: the spear patrols' sight, the club's shockwave), `En_Xc` 6 (about 225) | the effects `En_Mb` uses; `En_Xc`'s song half (types 2's actions) | about 1,850 |
| **The twisted corridors** | `Bg_Mori_Hineri` (288): the corridor's mesh rolled by Link's z, swapped on flags 0x13 and 0x18 with one-points 3260 and 3261 | `CAM_SET_CS_C`, dynapoly | the actor and its one-points |
| **The temple's set pieces** | `Bg_Mori_Elevator` (282), `_Bigst` (269), `_Rakkatenjo` (256), `_Idomizu` (194), `_Kaitenkabe` (187), `_Hashira4` (186), `Obj_Blockstop` (69), `SDC_FOREST_TEMPLE` (about 30) | water boxes, crushing, the push state | about 1,750 |
| **The Poe sisters** | `En_Po_Sisters` (1,433), `Bg_Po_Event` (647: the paintings, Amy's timed blocks), `Bg_Po_Syokudai` (197) | the intro's one-point 3140, the HUD timer | about 2,280 |
| **The temple's enemies** | `En_Test` (2,080, whole: shared code), `En_Bb` (about 1,180 of 1,377: Blue and Green; Red and White later), `En_Floormas` (1,155), `En_Wallmas` (677) | the grabs, respawn | about 5,100 |
| **One-points** | 3150 to 3240, 3260, 3261, 6010, 9704 | 3140 and earlier phases' | about 300 |
| **Phantom Ganon** | `Boss_Ganondrof` (1,549), `En_fHG` (748: the horse, the intro cutscene), `En_Fhg_Fire` (770: lightning, the energy ball sent back by the sword, a bottle or a spin) | the electrified knockback, the flash | about 3,070, plus **textures changed at run time** (his death's decay writes limb textures each frame: no precedent in the renderer) |
| **The medallion** | `Door_Warp1`'s adult warp (about 260), `Demo_Sa` 0 and 1's Forest part and `gForestMedallionCs` (about 320), `Obj_Dekujr` (193) and the Sprout's cutscenes | the child warp, the Chamber's `chamberCutsceneNum`, `Demo_Effect` | about 770 |
| **Optional** | `Obj_Mure3` (205: Kokiri Forest's rupee ring), `Obj_Ice_Poly` (220: room 14's frozen switch, now a placeholder that flips at once), `Obj_Bean`'s adult lift (#29) | | about 425 plus #29 |

### Sizes

| Part | Lines of C (about) |
|---|---|
| The hookshot and Dampé's race | 620 + 700 = 1,320 |
| The Lost Woods and the meadow (Moblins, the Minuet, `Obj_Mure3`) | 1,850 + 205 = 2,055 |
| The temple's front and the bow (`En_Test`, `Bg_Mori_Bigst`, `_Elevator`, the scene's draw) | 2,080 + 269 + 282 + 30 = 2,660 |
| The courtyards and the corridors (`Bg_Mori_Hineri`, `_Idomizu`, `_Hashira4`, `_Rakkatenjo`, `_Kaitenkabe`, `Obj_Blockstop`, `Obj_Ice_Poly`, `En_Bb`, `En_Wallmas`, `En_Floormas`, one-points) | 288 + 194 + 186 + 256 + 187 + 69 + 220 + 1,180 + 677 + 1,155 + 300 = 4,710 |
| The Poe sisters | 2,280 |
| Phantom Ganon and the medallion (the exit) | 3,070 + 770 = 3,840 |
| Not this phase (proposed): `En_Hs`, `En_Mm2` (Phase 20); `En_Bb`'s Red and White | (674 + 195) |

**About 16,900 lines:** more than twice Phase 13 (7,100), the largest phase yet. The temple alone
is about 13,500.

### Things to check (for the user, or at a milestone's start)

- **Fights everywhere.** Dampé's race, the Moblins, two Wolfos, four Stalfos (three for the bow),
  four Poe sisters (Meg with her fakes), Bubbles, a Wallmaster, a Floormaster, a ReDead, Phantom
  Ganon. By the standing rule none is scripted: the runs start past them on debug presets with the
  flags set (0x15 for the bow's chest, 0x1D to 0x1F for Meg, the boss key), and the fights are the
  hand test's.
- **The temple is long:** 23 rooms, six keys. Runs per area from debug starts, never end to end.
- **`Rand`:** the paintings' order (`Bg_Po_Event`) and Meg's fakes. Their cases' sequences depend on
  the call order.
- **Mido needs Saria's Song** (`EVENTCHKINF_0A` isn't set by the story so far): a Lost Woods run
  plays it, as GAME-08's `Route::Meadow` start does for a child.
- **To settle at the temple's milestones:** how Link reaches room 0's switch and the Song of Time
  blocks in rooms 4 and 6; flag 0x13 is shared by room 5's timer and room 11's crystal switch (which
  corridor state the boss key wants); the meadow tree's hookshot polygon.
- **Goldens:** adult Kokiri Forest cases (`spot04_*`, `golden.py:47-60`) may move with `Obj_Mure3`
  (its ring is within about 840 of spawns 4 and 11); `blue_warp*` share `Door_Warp1`'s draw with the
  adult crystal; hookshot edits to the shared upper actions could touch the slingshot cases. Child
  cases elsewhere shouldn't move. `temple_of_time_runs.rs:178` relies on the save's default
  `chamberCutsceneNum` (`CHAMBER_CS_FOREST`): once `Demo_Sa` 0 acts, the Chamber's layers 0 to 3
  play Saria's scene.
- **The pack:** `object_mori_objects` with `object_mori_tex`, the corridors' four meshes and
  collision, the Poe sisters, the Stalfos, the Bubbles, the Wall- and Floormasters, Phantom Ganon
  (with the decay's textures), his horse (a skin, like Epona's), the warp crystal, Saria's Chamber
  model: bakes and a format bump per milestone that adds them.

## The proposal

Six milestones in the story's order; the exit at 6.

| # | Milestone | What | Run (short debug starts; no fights) |
|---|---|---|---|
| 1 | **The hookshot and Dampé's race** | `Arms_Hook` and Player's hookshot whole (aim, fire, pull, the reticle, `Camera_Special5`, the attachments), `En_Po_Relay`, `En_Honotrap` (whole, or its flame drop and the rest a placeholder: a call), the race's chest | (a) Dampé's grave from `epona-won` at its start: talk, the race begins, the first door. (b) A debug `hookshot` preset at the chest: hookshot an `Obj_Hsblock` and a wall target, pulled across. The race is the hand test's |
| 2 | **The Lost Woods, the meadow and the Minuet** | `En_Mb` whole, `En_Xc` 6 (the Minuet's teaching), `Obj_Mure3`; the Minuet's warp; the hookshot to the temple's ledge | (a) The Lost Woods from `hookshot`: Mido, Saria's Song, the meadow. (b) A debug start at the meadow's top: Sheik's Minuet (`EVENTCHKINF_50`). (c) The Minuet's warp. (d) The hookshot up to the temple's door, into it. The maze past the Moblins is the hand test's |
| 3 | **The temple's front and the bow** | `En_Test` whole, `Bg_Mori_Bigst`, `Bg_Mori_Elevator`, `SDC_FOREST_TEMPLE`, the scene's one-points; the bow tested (on foot, first person, from horseback) | (a) The entrance to the lobby (the sisters' intro, ported). (b) A debug start in room 6 with flag 0x15: the bow's chest; arrows at a target, an eye switch |
| 4 | **The courtyards and the twisted corridors** | `Bg_Mori_Hineri`, `_Idomizu`, `_Hashira4`, `_Rakkatenjo`, `_Kaitenkabe`, `Obj_Blockstop`, `Obj_Ice_Poly`, `En_Bb` (Blue, Green), `En_Wallmas`, `En_Floormas`, their one-points | (a) The well drained. (b) Room 11's blocks and the corridor straightened (one-point 3260), the boss key's chest. (c) The basement's rotating wall and a gate. The grabs and the fights are the hand test's |
| 5 | **The Poe sisters** | `En_Po_Sisters`, `Bg_Po_Event`, `Bg_Po_Syokudai` whole; the elevator's Meg state | (a) A painting shot with arrows (Joelle appears). (b) Amy's block puzzle under its timer. (c) With 0x1D to 0x1F, Meg appears at the lobby's torches. The sisters' fights are the hand test's |
| 6 | **Phantom Ganon and the Forest Medallion (the exit)** | `Boss_Ganondrof`, `En_fHG`, `En_Fhg_Fire` whole (the death's texture decay: a call); `Door_Warp1`'s adult warp; `Demo_Sa` 0 and 1's Forest part (BACKLOG #110 in part); `Obj_Dekujr` and the Sprout's cutscenes | (a) The boss room: his intro and title card. (b) A debug start with him beaten: the blue warp, the Chamber (Saria, the medallion: `EVENTCHKINF_48`), Kokiri Forest's Sprout: **the exit**. The fight is the hand test's |

The scope rule is as before: every actor whole, except the parts another phase's init chooses
(their init's checks, then a placeholder): `En_Bb`'s Red and White, `En_Honotrap`'s types placed in
the Shadow Temple (if the user's call 3 says so), `En_Hs` and `En_Mm2` (Phase 20).

**The helpers' share** (at most two at once): milestone 1's Player and camera side stays in the main
session (Player is one system); `En_Po_Relay` and `En_Honotrap` to a helper. Then one helper per
enemy family (`En_Mb`; `En_Test`; `En_Bb` with `En_Wallmas` and `En_Floormas`; the Poe sisters;
Phantom Ganon), the `Bg_Mori_*` set pieces to another, the main session keeping the runs, presets,
goldens, docs, the one-points and the renderer.

**Presets**, chained (debug starts marked):
- `hookshot`: `epona-won` with the race's chest opened (`ITEM_HOOKSHOT`).
- `minuet`: `hookshot` with Mido past (`EVENTCHKINF_0A`) and the Minuet (`EVENTCHKINF_50`).
- `forest-bow` (debug): inside the temple with the bow and the flags to room 6.
- `forest-boss` (debug): the boss key and the boss door's flags.
- `forest-medallion`: the medallion (`EVENTCHKINF_48`); **Phase 15's start**. It replaces the debug
  `prelude`'s stand-in flag.

## The user's calls

2026-10-27: the user took the breakdown and every recommendation below:
- **The split:** six milestones in this order, the exit at 6.
- **`En_Hs` and `En_Mm2`:** Phase 20's, placeholders here; COVERAGE corrected.
- **`En_Honotrap`:** whole now.
- **Phantom Ganon's decay:** as the C, the renderer updating a texture at run time.
- **Dampé's race and the meadow's maze:** their starts and ends in runs, the rest the hand test's.
- **`Obj_Bean`'s adult lift (#29):** later.

The calls as proposed:

1. **The split:** six milestones as proposed, the exit at 6. Recommended. Or: four (fold 4 and 5
   into one temple milestone, and 1 into 2), each larger.
2. **`En_Hs` and `En_Mm2` to Phase 20** (Grog's trade, the marathon), placeholders here; COVERAGE
   corrected. Recommended. Or: port them now (674 lines) as Phase 14's breadth.
3. **`En_Honotrap`:** whole now (559: its other types are placed in the Shadow Temple, Phase 17).
   Recommended, as `En_Test`'s and `En_Floormas`' shared code. Or: the flame drop only, the rest a
   placeholder until Phase 17.
4. **Phantom Ganon's death decay** (textures rewritten each frame): ported as the C does, with the
   renderer updating a texture at run time. Recommended (follow the original). Or: a fade standing
   in, logged in BACKLOG.
5. **Dampé's race and the meadow's maze in the runs:** their starts and ends in runs, the race and
   the maze the hand test's (as Ingo's race). Recommended.
6. **`Obj_Bean`'s adult lift (BACKLOG #29):** later (Phase 20's breadth: its rupees are optional).
   Recommended. Or: in milestone 2 with Kokiri Forest's bean.

## Milestone 1: The hookshot and Dampé's race

**Status:** done: built and hand-tested 2026-10-27 ("these all work"), committed and pushed on the
user's ask. The hand tests ran on the window's DirectX 12 fix (ADR 0097).

**The answer:**
- **Dampé's race:** in his grave Dampé's ghost greets Link and races him through the rooms behind
  the doors he opens as he passes (switches 0x35 to 0x37), dropping fire on the way; lag behind and a
  door shuts. The first race won: room 4's chest, the hookshot. Later races, under 60 seconds: a
  piece of heart; the best time kept (`HS_DAMPE_RACE`).
- **The hookshot:** C raises it in first person, the red reticle where its line meets a surface.
  Let go, the hook flies 13 frames at 20, its chain from the hand. It holds in a surface that takes
  it (a target's band, a wall) and pulls Link there on the hookshot's camera; a moving platform
  carries it. It catches what takes it (a hookable enemy, an item: dragged back). Anything else
  throws it back with a spark.
- **The eyes** (`En_Honotrap`, whole: the Shadow Temple's flame eyes, Phase 17's scene) and Dampé's
  falling fire.

ADR 0096. Scripts: `scripts\run\game-hookshot.bat` (menu 190: empty, `post`, `eyes`),
`test-hookshot.bat` (191), `sandbox-hookshot.bat` (192).

### What was built

- **`Arms_Hook`** whole: its wait on the hookshot, the shot, the hit on an actor (attached, pulling
  Link or dragged back) or a surface (`SurfaceType_CanHookshot`: attached to its bg actor if any,
  pulling Link; else the spark and the reflect sound), the return, the cancel, its draw (the tip, the
  chain stretched to the hand) and its quad at draw time.
- **Player's hookshot:** `Player_InitHookshotIA`'s spawn (`PlayRequest::SpawnHeldHook`),
  `Player_DestroyHookshot`, the flight's start in `Player_UpdateUpperBody`,
  `Player_Action_HookshotFly` and its landing, `CAM_MODE_HOOKSHOT_FLY` on the hook,
  `func_8008F128` (the hookshot with no hook), the right hand's post-limb part (`unk_3C8`, the hook
  at the tip) and the reticle (`Player_DrawHookshotReticle`).
- **The camera, collision and draw:** `Camera_Special5`, `SurfaceType_CanHookshot`,
  `BgCheck_AnyLineTest3`, `SETUPDL_7`; bakes for the tip, the chain and the reticle (billboarded:
  its list's segment-1 matrix). Pack format 37 (`out/data29`).
- **`En_Po_Relay`** and **`En_Honotrap`** (the helper's) whole; `Item_DropCollectible2`.
- **The runs** (`playthrough.rs`; tests in `hookshot_run.rs`): `Route::DampeRace` and
  `Route::Hookshot`; `Who::DampeGhost`; the aiming tasks find the hookshot's button
  (`slingshot_button`). The preset **`hookshot`** (`epona-won`, Dampé's chest opened, the hookshot on
  C-Left: the story leaves the buttons to the player).
- **Names:** the editor's "Hookshot's hook", "Dampé's ghost", "Fire eye trap"; `ootx`'s "Dampe
  (ghost)".

### Results

- The runs (frames):
  - Dampé's race (`dampe-race`): 1,002. The race on 361 (his greeting read, `EnPoRelay_Race`, the
    timer counting up), the first door 1,002 (switch 0x35 at his eighth point, Link standing on the
    landing);
  - the hookshot (`hookshot`): 77. Raised in first person 32, pulled 46 (the hook in the post's
    upper band), caught on its top edge 52, up onto it 77; `CAM_MODE_HOOKSHOT_FLY` while he flies.
- The texts: Dampé's greeting 0x5041.
- Tests: `hookshot_run` (2); `hookshot` (2: the pull to the post, the lower band throwing it back);
  `en_po_relay` (9) and `en_honotrap` (6), the helper's. 1,882 tests pass, 3 ignored (two the level
  editor session's); `cargo test -p layering` passes; no warnings.
- Goldens: all 400 of `out/golden_base47`'s hashes the same (no earlier case uses the hookshot or
  enters Dampé's grave; the slingshot's cases, which share the aiming code, unchanged). 5 new cases
  (7 hashes): 407 hashes, 297 cases, the same over two runs (`out/golden_base48`).

### Corrections and findings

- **The stone post's band:** only its upper polys (from about 550, 70 above its base) take the
  hookshot; the lower ones throw it back, as in the original.
- **After the flight Link catches the post's top edge** (`func_80837B9C`'s ledge check, as the C)
  rather than landing beside it; the run climbs up.
- **Dampé's offer is auto-accepted** within 250 of him: no A to start the talk.
- **Dampé reaches the first door 641 frames after the race starts** with Link standing still; the
  door stays open 160 frames.
- **Three `@bug (game)`s** (the helper's): Dampé's speed follows Link's (the decomp's note); after his
  last point the race reads `sPathPoints[28]`, past the array; an eye's angle check doesn't wrap, so
  an eye facing -0x8000 never sees Link just east of ahead.
- **The helper's share went as proposed** (`En_Po_Relay` and `En_Honotrap`, in parallel with the
  hookshot).

### Known gaps

- The race itself (to the chest, the heart piece) is the hand test's (the user's call 5).
- The reticle draws in the XLU list with `SETUPDL_7`'s mode, not in `OVERLAY_DISP` (the same: no
  depth compare, after the opaque list).
- `Actor_SwapHookshotAttachment` waits for `En_Fd` (Phase 15). No rumble.

### How to check

```
scripts\run\test-hookshot.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-hookshot.bat
```

### By hand

`scripts\run\game-hookshot.bat` (menu 190). WASD the stick (Shift walks), Space A, E B, Q Z, J/K/L
the C buttons, I C-Up, Enter Start.

1. **Empty** (Dampé's grave, `epona-won`): walk down to Dampé; his greeting opens. Race him: follow
   through the doors he opens before they shut, past the fire he drops. At the end, the chest: the
   hookshot. Race again (talk to him at the start): under 60 seconds, a piece of heart.
2. **`post`** (Goron City, the hookshot on C-Left, on the ledge by the stone post): J raises the
   hookshot in first person; the stick aims (the red reticle); let go of J: the hook flies, its chain
   behind it. At the post's top band Link is pulled and catches its edge; lower down the hook sparks
   and comes back. Try walls, a pot, a rupee, an enemy.
3. **`eyes`** (the Shadow Temple's first room, with the hookshot): the eyes open as Link nears and
   shoot fire at him; hit one to break it.
