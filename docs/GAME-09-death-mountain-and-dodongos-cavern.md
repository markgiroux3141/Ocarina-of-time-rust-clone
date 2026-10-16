# GAME-09: Death Mountain, Goron City and Dodongo's Cavern

**Goal:** Phase 10 of the [roadmap](ROADMAP.md): child Link with Zelda's letter up Death Mountain
Trail to Goron City, Darunia's dance and the Goron's Bracelet, the bomb flower that opens Dodongo's
Cavern (Master Quest), the cavern and its bomb bag, King Dodongo, the blue warp and the Goron's
Ruby; the summit's Great Fairy and the magic meter. Exit: the Goron's Ruby.

**Status:** surveyed, proposed and decided (2026-10-15; the user's calls at the end of this doc).
The phase starts from the `ranch-epona` preset (GAME-08's exit review).

| # | Milestone | Status |
|---|---|---|
| 1 | Bombs: `En_Bom`, `En_Bombf`, Player's explosives, BACKLOG #33 | done (2026-10-15) |
| 2 | The trail and Goron City to the bracelet, and the cavern's mouth | done (2026-10-15) |
| 3 | Dodongo's Cavern's route | next |
| 4 | King Dodongo and the Goron's Ruby (the phase's exit) | |
| 5 | The rest of the cavern | |
| 6 | The summit: the Great Fairy and magic | |

## The survey

Checked against the decomp (52a510f) and the pack (`out/data25`), not against earlier notes. Line
numbers are the decomp's; `ootx scene-info` was run on every layer of the five scenes.

### The scenes and their layers

| Scene | Layer | Cutscene | Who meets it |
|---|---|---|---|
| `spot16` (Death Mountain Trail) | 0, 1 | | a child (layer 1's header is 0: the day's placements) |
| | 2, 3 | | an adult (Phase 15) |
| | 4 | `gDMTKokiriEmeraldPart6Cs` | Phase 7 (done) |
| | 5 | `gDMTGoronRubyCs` | **this phase**: King Dodongo's blue warp (`z_door_warp1.c:516-525`) |
| | 6 | `gDMTUnusedCs` | nobody: camera splines only; `CS_INDEX_2` is never set for the trail (the map select only) |
| | 7 | `gDMTCreditsShowGoronCityCs` | the credits (Phase 19) |
| | 8 | `gDMTCreditsSixSagesCs` | the credits (Phase 19) |
| `spot18` (Goron City) | 0, 1 | | a child (layer 1 = layer 0) |
| | 2, 3 | | an adult (Phase 15) |
| | 4 | `gGoronCityDaruniaDancingCs` | nobody in play: `En_Du` starts it only with `cutsceneIndex >= CS_INDEX_0` (`z_en_du.c:311-314`), which only the map select sets |
| | 5 | `gGoronCityCreditsCs` | the credits (Phase 19) |
| `daiyousei_izumi` (the Great Fairy) | 0 to 3 | | everyone (one set of placements) |
| | 4 | `gGreatFairyMagicCs` | the debug route only: in play `Bg_Dy_Yoseizo` starts the script itself on layer 0 (`z_bg_dy_yoseizo.c:325-348`) |
| | 5 | `gGreatFairyDoubleMagicCs` | the same; double magic (the crater's fountain, an adult: Phase 15) |
| | 6 | `gGreatFairyDoubleDefenseCs` | the same; double defence (outside Ganon's Castle: Phase 19) |
| `ddan` (Dodongo's Cavern, MQ) | 0 to 3 | | everyone (one set; no cutscene layer) |
| `ddan_boss` (King Dodongo) | 0 to 3 | | everyone (one set) |

**Corrections to `scripts/coverage.py` (proposed):**
- `SCENES`: `spot16` `{4: 7, 8: 19}` becomes `{4: 7, 6: None, 7: 19, 8: 19}`; `spot18` gains
  `{5: 19}`; `daiyousei_izumi` gains `{5: 15, 6: 19}`. No actor's first phase moves with these (the
  layers' actors are all placed in the child layers too).
- `ACTORS`: `"En_Fire_Rock": 10`. `En_Encount2` (placed on the trail) spawns it for a child too
  (`z_en_encount2.c:65-67` kills the spawner only for an adult with `EVENTCHKINF_49`; the spawn is
  `:246-247`); the table counts it in Phase 19 now, from Ganon's Castle's placements.

### The route a child takes

From `ranch-epona` (Zelda's letter already shown: Kakariko's gate is open).

1. **Kakariko's gate** (`Bg_Gate_Shutter`, ported) to `ENTR_DEATH_MOUNTAIN_TRAIL_0`; the first entry
   plays `gDMTIntroCs` (`z_demo.c:124`, `EVENTCHKINF_A1`; camera, the title card, a stop: all
   ported).
2. **The trail** (spot16 room 0): four red Tektites (`En_Tite` -1) at the bottom; the bombable
   wall `Bg_Bombwall` 0x8001 over a Gold Skulltula; the flag (`En_Hata`); the fairy-hint Goron
   (`En_Go2` type 0xC); the cavern's mouth with its boulder (`Bg_Spot16_Bombstone` 0x04FF), the
   cavern Goron (type 6), the soft soil; above it the cliff with the bomb-flower Goron (type 4,
   path 0) and the trail's one bomb flower (`En_Bombf` at (-1397, 1408, -646)); the rolling Goron
   (type 5, path 1); Goron City's door (exit 2, `ENTR_GORON_CITY_0`).
3. **Goron City** (spot18; its intro `gGoronCityIntroCs`, `z_demo.c:128`, `EVENTCHKINF_A6`):
   - Darunia's door, `Bg_Spot18_Shutter` 0x001B, opens on switch 0x1B, which the song spot
     `En_Okarina_Tag` 0x1C9B (type 7, Zelda's Lullaby; ported) sets
     (`z_en_okarina_tag.c:137-147, 164-166`).
   - **Darunia** (`En_Du`, room 1): his texts (0x301A, 0x301B), then with the ocarina out
     `Message_StartOcarinaSunsSongDisabled(OCARINA_ACTION_CHECK_SARIA)`; Saria's Song starts
     **`gGoronCityDaruniaCorrectSongCs`**, another song `gGoronCityDaruniaWrongSongCs`
     (`z_en_du.c:353-375`), on layer 0 in place. Both end with `CS_MISC_STOP_CUTSCENE`, no
     terminator. Then text 0x301C and **`Actor_OfferGetItem(GI_GORONS_BRACELET)`** the first time
     (`z_en_du.c:514-557`); no event flag: the strength upgrade is the state.
   - Five more Gorons (types 7 to 0xB, killed for an adult without the Fire Medallion,
     `z_en_go2.c:1576-1584`), the big rolling Goron (type 0: a bomb stops him; with the 20 bomb bag
     he gives the 30, `z_en_go2.c:333-358`), Medigoron behind a bombable wall (`En_Gm`: a child gets
     texts only, `z_en_gm.c:106-116`), the spinning urn (`Bg_Spot18_Basket`, lit torches and a bomb
     dropped in it: a heart piece), the statue (`Bg_Spot18_Obj`, still for a child), the bronze
     boulders (`Obj_Hamishi`: the hammer's), the hookshot post (`Obj_Hsblock`), 29 silver rocks
     (`En_Ishi` 0xFF01: the silver gauntlets').
4. **The cavern's mouth:** a child lifts the cliff's bomb flower only with the bracelet (Player
   strains at it with `Player_GetStrength() <= PLAYER_STR_NONE`, `z_player.c:5633-5641`) and drops it
   onto the boulder; the boulder watches a held bomb flower and starts one-point cutscene 4180 as
   it falls (`z_bg_spot16_bombstone.c:431-449`); the blast breaks it, sets switch 4 and
   `EVENTCHKINF_23` (`:466-474`).
5. **Dodongo's Cavern, Master Quest** (below): the bomb bag, the skull's eyes, the gravestone over
   the boss path's switch, the fall to King Dodongo.
6. **King Dodongo** (below), the blue warp: `EVENTCHKINF_25`, `Item_Give(ITEM_GORON_RUBY)`,
   `ENTR_DEATH_MOUNTAIN_TRAIL_0` with `CS_INDEX_1` (`z_door_warp1.c:516-525`): layer 5,
   `gDMTGoronRubyCs` (Darunia, three Gorons, the ruby, Link fleeing down the slope), its terminator
   12 to `ENTR_DEATH_MOUNTAIN_TRAIL_5` (`z_demo.c:735-739`; the port's table has the row). **The
   phase's exit.**
7. **The summit** (with bombs now): the falling rocks above y 1500 (`En_Encount2`, `En_Fire_Rock`),
   three Skullwalltulas, the Great Fairy's wall `Bg_Bombwall` 0x8003, exit 5 to her fountain.
   Zelda's Lullaby on the Triforce (`En_Okarina_Tag` 0x1CB8, switch 0x38, ported) brings her; her
   reward is chosen by `play->spawn` (`z_bg_dy_yoseizo.c:95-98`), spawn 0 the magic; she starts
   `gGreatFairyMagicCs` (her cues 2, 3, 0x13 an orange `Door_Warp1` at Link, 0x0A the magic:
   `isMagicAcquired`, `magicFillTarget` 0x30, `z_bg_dy_yoseizo.c:745-776`). The fountain's exit is
   `0x7FFE`, a return entrance (`sReturnEntranceGroupData`, `z_player.c:5148-5186`) to
   `ENTR_DEATH_MOUNTAIN_TRAIL_4`.
8. **The owl** (`En_Owl` 0x0213, type 8, ported): without magic text 0x3062 and he stays; with it
   0x3063, and he flies Link to Kakariko by `gDMTOwlCs` (terminator 66,
   `ENTR_KAKARIKO_VILLAGE_14`) (`z_en_owl.c:638-653`). No test covers him in this scene yet.

The night (layer 1) places the day's actors in both overworld scenes, and none of them checks the
time (only `En_Sw`'s type 2, not placed here).

### Dodongo's Cavern, Master Quest

17 rooms (0 to 16), 23 transition actors, no entrance cutscene, **no small keys, no boss key and
no boss door**.

- **The way in:** room 0's entrance wall, `Bg_Breakwall` 0x0007 (type 0, `BWALL_DC_ENTRANCE`), takes
  an explosion; its first break sets `EVENTCHKINF_B0` and plays `gDcOpeningCs` with the title card
  (ported, `bg_breakwall.rs`). Player shows the cavern's title card on later entries only with
  `EVENTCHKINF_B0` (`z_player.c:10770`). The two bomb flowers beside the spawn are the only
  explosives before the wall: **the bracelet is required to enter.**
- **The chests:** room 0, `En_Box` 0x0644 at (869, 80, -920): **`GI_BOMB_BAG_20`** (0x32,
  `item.h:371`), a plain big chest in the lobby; room 0's map (behind bomb wall 0xA000); room 5's
  compass (room clear: three Dodongos); room 6's Deku Shield (room clear: five Gohma larvae);
  room 9's blue rupee; room 14's Hylian Shield under a gravestone.
- **What the route must do** (from the flags): (1) break the entrance wall; (2) light both of the
  skull's eyes with explosions (`Bg_Dodoago` 0x001A: a blast on each side, one-point 3065 each,
  then 3380 and the jaw; `z_bg_dodoago.c:170-218`); the "bomb catcher" stops an `En_Bom` with
  params 0 dropped through the bridge's holes over the eyes (`:298-309`); (3) pull room 7's
  gravestone (`Bg_Haka`: a child may, outside the graveyard by day and Lake Hylia,
  `z_bg_haka.c:83-96`) off floor switch 0x20 (`Obj_Switch` 0x2000), which unbars door 20 (0x00A0,
  `SHUTTER_FRONT_SWITCH`); (4) fall into room 16's pit: exit 2, `ENTR_DODONGOS_CAVERN_BOSS_0`.
- **Two ways to the skull's bridge** (the lobby's upper level, y 531):
  - *A*, the short one: bomb room 0's pit boulder (`Obj_Bombiwa` 0x000E) off floor switch 0x0A;
    the pillar `Bg_Ddan_Jd` 0x000A then shuttles to y 520 (one-point 3060).
  - *B*, the loop: room 4 (Poes, bomb-flower chains) or room 1 (the slingshot's eye switch) into
    room 3 (two Lizalfos pairs), room 12 (fire walls) and room 9 to the bridge.
  Which one a child can climb isn't settled: `ootx` doesn't dump collision. The breadth-first
  search over the collision (GAME-08's route aid) settles it at the milestone's start.
- **Not needed by the route:** the Song of Time (room 15's 15 blocks hide a Gold Skulltula), any
  other song, the Lens, magic, the hookshot. The slingshot only for route B's room 1 and room 8's
  crystal; Deku Sticks only for room 2's flower fuse, room 9's timed torches (room 6) and room 10's
  web.
- **Temp flags:** the boss path's 0x20 and the silver rupees' 0x25 are lost on leaving the scene
  (`z_actor.c:643-649`); 0x38 and up on every room change (`z_actor.c:3064`). `Bg_Ddan_Jd` 0x00FF
  reads flag 0xFF through a shift the console masks to 5 bits (`z_bg_ddan_jd.c:74-77`): the port
  has to mask it the same way.
- **The scene's draw config** (`Scene_DrawConfigDodongosCavern`, `z_scene_table.c:279-314`): the
  entrance texture by `nightFlag`, the lava floor flipping through eight textures by
  `gameplayFrames`, two scrolls, and the eyes' glow from `roomCtx.drawParams[0..1]`, which
  `Bg_Dodoago` writes.

### King Dodongo, the warp and the Ruby

- **The room** (`ddan_boss`): Link lands at spawn 0, bombs `Bg_Breakwall` 0x4001 (type 2, the floor,
  ported) and drops into the arena; four bomb flowers in its corners, a chest of five bombs.
- **`Boss_Dodongo`** (1,738 + 240 data): Gohma's pattern (ADR 0050) for the intro and the death on
  his own sub camera, the title card, the heart, `Door_Warp1` as his child, the cleared room at
  init, the dissolve into object RAM (ADR 0051). New: he **writes a room texture** (the lava floor,
  `gDodongosCavernBossLavaFloorTex`, from textures in his overlay's data, one of which he writes
  too); `EffectSsGMagma` and `GMagma2`; his own 80-particle system; `Play_SetCameraAtEyeUp` (the
  camera's up vector isn't modelled, `camera.rs`); the `MREG(64)` screen fill (`z_play.c:1380`);
  `En_Bdfire` (233); limb wobble; his title card (128×120 in this ROM). He eats any
  `ACTORCAT_EXPLOSIVE` within 40 of his mouth during the inhale (a bomb, or a carried bomb
  flower's body), reading no `En_Bom` field. The lava's damage is Player's hot floor (ported).
- **`Door_Warp1`'s branch** for his room is logged in the port (`door_warp1.rs:409-413`), and the
  logged branch still starts the transition with a stale entrance: fix with the branch.
- **The Ruby's cutscene** (`gDMTGoronRubyCs`, 51 commands): every command, cue slot and Player cue
  is already handled by the port's cutscene system (text type `CS_TEXT_GORON_RUBY`, terminator 12,
  the transitions). The actors aren't: `Demo_Du` (`DEMO_DU_CS_GORONS_RUBY`, lines 318-716 of
  1,069) and `Demo_Go` (383, all of it this chain's). `Demo_Effect`'s light and jewel are ported.
  `Bg_Spot16_Bombstone` never spawns in layer 5 (its object isn't in the room's list).

### What's already ported and reused

`Bg_Breakwall` (all four types), `Obj_Bombiwa`, `Obj_Kibako2`, `Obj_Switch` (floor, eye, crystal),
`Obj_Syokudai` (golden, timed, wooden), `Bg_Haka`, `Obj_Timeblock`, `Obj_Oshihiki`, `En_St`, `En_Sw`,
`En_Goma`, `En_Firefly`, `En_Dekunuts`, `En_Shopnuts`, `En_Karebaba`, `Bg_Ydan_Sp`, `Elf_Msg`,
`En_Wonder_Item`, `En_Wonder_Talk2`, `En_Okarina_Tag`, `En_Gs`, `En_Owl`, `Door_Shutter`, `En_Holl`,
`En_Box`, `Item_B_Heart`, `Door_Warp1` (but the branch), `Demo_Effect`, `Bg_Spot16_Doughnut`,
`SDC_DEATH_MOUNTAIN_TRAIL`. Player's lift, carry, throw and put-down (ADR 0064), `Player_GetStrength`,
`Item_Give` for the bomb bag, bombs and the bracelet, `Inventory_ChangeAmmo`, the C buttons' ammo
digits, the explosions' neighbours (`func_80033684`, `Actor_GetCollidedExplosive`'s callers' side),
`EffectSsBomb2`, `Blast`, `Dust`, `DeadSound`, `DeadDb`, `Hahen`, `EnIce`, `KiraKira`, the hot floors
and the hot rooms' timer.

### The systems

- **Bombs.** `En_Bom` (396) and `En_Bombf` (523) aren't ported (BACKLOG #33: the Peahat's death and
  `En_Heishi2`'s bomb are placeholders; `En_Wf`, the gossip stones, the walls, rocks and crates wait
  on them). A bomb flower explodes itself (it doesn't spawn an `En_Bom`); picking one hands the carry
  from the flower to its new bomb with `func_8002F5C4` (`z_actor.c:1908-1921`, not ported); a bomb or
  flower that blows in Link's hand clears his held-actor fields itself (no `PlayerIface` write for it
  yet). Player: `Player_InitExplosiveIA` is logged (`player.rs:7477`) and the bombs' upper-body action
  still maps to the default (`player.rs:6716`), though carrying is ported now. The fuse's sparks are
  `Effect_Ss_G_Spk` (153, not ported).
- **The bracelet:** given and read already; Link's child bracelet (`gLinkChildGoronBraceletDL`,
  `z_player_lib.c:1160-1163`) isn't drawn.
- **The magic meter:** `Magic_Fill`, `_Reset`, `_RequestChange`, `_Update`, `_DrawMeter` (387 lines of
  `z_parameter.c`), `Interface_Update`'s start (19), the magic jars' half of `Item_Give` (40, logged);
  the save context's `magicState`, `prevMagicState`, `magicCapacity`, `magicFillTarget`,
  `magicTarget` aren't in the port. `gMagicMeterFillTex` is loaded by the C as I4 16×16 though the
  XML says IA8 8×8. BACKLOG #49 (the songs' `Magic_Reset`) closes with it.
- **The Great Fairy:** `Bg_Dy_Yoseizo` (1,039), `En_Dy_Extra` (142, the revisit's beam), the orange
  warp's fade (`func_809998A4`, 7 lines, logged in `door_warp1.rs`), one-point 8603 (the revisit),
  the return entrances (`sReturnEntranceGroupData`: **Link can't leave the fountain** until they're
  ported, `player.rs:8268-8272`).
- **The magic spin attack:** `En_M_Thunder` (415) is what Player spawns for every spin attack's
  effects (`func_80837530`, logged at `player.rs:7604`); with magic a charged spin uses it.
- **Draw configs:** `SDC_DODONGOS_CAVERN` (36, above), `SDC_GORON_CITY` (21: an OPA scroll and an
  XLU texture by `nightFlag` on the same segment 8), `SDC_GREAT_FAIRYS_FOUNTAIN` (23, two scrolls).
- **One-point cutscenes, none ported:** 4180 (the boulder), 4210 and 4220 (the urn), 3050 (the
  falling stairs), 3060 (the pillar), 3065 and 3380 (the eyes and the jaw), 8603 (the fairy's
  revisit).
- **Effects not ported that these actors spawn:** `G_Spk` (153), `D_Fire` (140, the Dodongos'
  breath), `Dead_Dd` (154) and `Dead_Ds` (130, the Beamos), `G_Magma` (67), `G_Magma2` (139);
  `G_Fire` (62, the hot floor's footprints) optional. `EffectBlure` (the Lizalfos's sword trail;
  Link's and the Skulltula's too) is the roadmap's rendering debt.
- **The scene's wind** (`SCENE_CMD_ID_WIND_SETTINGS`, `z_scene.c:480-489`): `En_Hata`'s flag reads
  `envCtx.windSpeed`; the port keeps a default direction and no speed (`env.rs`).

### Things checked that the notes asked about

- **Who gives a child the bomb bag:** the cavern's lobby chest (above). The big rolling Goron gives
  the 30 only to a child who has the 20 and stops him between waypoints 8 and 11.
- **The adult's:** the crater, Darunia's room's statue push, Link the Goron, Biggoron, Medigoron's
  sale, the silver rocks, the bronze boulders, the hookshot post (Phase 15). `En_Go2` type 0xD (the
  bazaar, after the Ruby) is Phase 20's shop.
- **Not settled:** which ledges a child climbs in the cavern (route A or B); which of the boss
  room's polygons are its hot floor; the texts' wording (no message dump in the decomp checkout).

### Sizes

| Part | Lines of C (about) |
|---|---|
| Bombs: `En_Bom`, `En_Bombf`, `Effect_Ss_G_Spk`, Player's explosives and `func_8002F5C4` | 396 + 523 + 153 + 110 |
| The trail: `En_Tite`, `Bg_Bombwall`, `Bg_Spot16_Bombstone`, `En_Hata` and the wind, one-point 4180 | 1,038 + 268 + 583 + 180 + 15 |
| Goron City: `En_Go2` (1,540 the child's, 570 the adult's), `En_Du`, `Bg_Spot18_Basket` and `_Futa`, `_Obj`, `_Shutter`, `En_Gm`, `Obj_Hamishi`, `Obj_Hsblock`, `SDC_GORON_CITY`, one-points 4210 and 4220, the child's bracelet | 2,113 + 655 + 552 + 301 + 149 + 350 + 220 + 196 + 25 + 60 |
| The cavern's route: `Bg_Dodoago`, `Bg_Ddan_Jd`, `En_Dodojr`, `SDC_DODONGOS_CAVERN`, one-points 3060, 3065, 3380 | 337 + 188 + 667 + 44 + 80 |
| The rest of the cavern: `En_Zf`, `En_Poh`, `En_Am`, `En_Dodongo`, `En_Vm`, `En_G_Switch`, `Bg_Hidan_Curtain`, `Bg_Gnd_Darkmeiro`, `Bg_Ddan_Kd`, `Obj_Elevator`, four effects, one-point 3050 | 2,362 + 1,210 + 992 + 988 + 568 + 594 + 275 + 220 + 199 + 138 + 560 + 30 |
| King Dodongo: `Boss_Dodongo`, `En_Bdfire`, `G_Magma`, `G_Magma2`, the room texture, the camera's up, `MREG(64)` | 1,978 + 233 + 206 + 100 |
| The Ruby: `Door_Warp1`'s branch, `Demo_Du` (520 the Ruby's, 1,257 whole), `Demo_Go` | 10 + 1,257 + 383 |
| The summit: the magic meter, `Bg_Dy_Yoseizo`, `En_Dy_Extra`, `SDC_GREAT_FAIRYS_FOUNTAIN`, the return entrances, the orange fade, one-point 8603, `En_Encount2`, `En_Fire_Rock` | 446 + 1,039 + 142 + 23 + 45 + 7 + 66 + 385 + 416 |
| `En_M_Thunder` (if in) | 415 |

About 23,000 lines whole (GAME-08's phase was about 9,000): 2.5 times Phase 9.

## The proposal

Six milestones in the story's order. The exit can't come first as it did in GAME-08: the Ruby
needs the bracelet, the bombs and the cavern. So the critical path to it comes first (milestones 1
to 4, the exit met at 4), then the breadth (5 and 6).

| # | Milestone | What | Run (from `ranch-epona` or the last one's end) |
|---|---|---|---|
| 1 | Bombs | `En_Bom`, `En_Bombf`, `Effect_Ss_G_Spk`, Player's explosives (`Player_InitExplosiveIA`, the carry, the in-hand blast's writes, `func_8002F5C4`), #33 (the Peahat's death, `En_Heishi2`'s bomb, the walls', rocks', crates', gossip stones' and Wolfos's reactions for real) | Hyrule Field's Peahat dies in its explosion; a debug start with a bomb bag (the cavern's) breaks a wall, a rock and a crate |
| 2 | The trail and Goron City to the bracelet | `En_Go2`, `En_Du` (the songs' scripts, the bracelet), `Bg_Spot18_Shutter`, `_Obj`, `_Basket`, `_Futa`, `En_Gm`, `Obj_Hamishi`, `Obj_Hsblock`, `En_Tite`, `Bg_Bombwall`, `Bg_Spot16_Bombstone`, `En_Hata` and the scene's wind, `SDC_GORON_CITY`, one-points 4180, 4210, 4220, the child's bracelet drawn. Perhaps in two steps (the trail and the city; the cavern's mouth) | Kakariko's gate, the trail, Goron City, the lullaby at Darunia's door, Saria's Song, the bracelet; back to the cliff, the bomb flower onto the boulder, into the cavern |
| 3 | Dodongo's Cavern's route | `SDC_DODONGOS_CAVERN`, `Bg_Dodoago`, `Bg_Ddan_Jd`, `En_Dodojr`, and whatever else the way the search finds needs (route B adds `En_Zf` and `Bg_Hidan_Curtain`); one-points 3060, 3065, 3380 | The entrance wall (its cutscene), the bomb bag, the pillar or the loop, the eyes, the jaw, the gravestone, the fall |
| 4 | King Dodongo and the Goron's Ruby | `Boss_Dodongo`, `En_Bdfire`, the two magma effects, the room texture's write, `Play_SetCameraAtEyeUp`, `MREG(64)`, the title card; `Door_Warp1`'s branch, `Demo_Du`, `Demo_Go` | From the boss room: the intro, bombs eaten, the death, the heart, the warp, `gDMTGoronRubyCs` to the trail's spawn 5: **the phase's exit** |
| 5 | The rest of the cavern | `En_Zf` (if not in 3), `En_Am`, `En_Poh`, `En_Dodongo`, `En_Vm`, `En_G_Switch`, `Bg_Hidan_Curtain` (if not in 3), `Bg_Gnd_Darkmeiro`, `Bg_Ddan_Kd`, `Obj_Elevator`; `D_Fire`, `Dead_Dd`, `Dead_Ds`; one-point 3050 | The compass (three Dodongos), the silver rupees and the falling stairs, the Armos room, the Poes |
| 6 | The summit: the Great Fairy and magic | The magic meter, `Bg_Dy_Yoseizo`, `En_Dy_Extra`, `SDC_GREAT_FAIRYS_FOUNTAIN`, the return entrances, the orange warp's fade, one-point 8603, `En_Encount2`, `En_Fire_Rock` (and `En_M_Thunder` if in) | Up through the falling rocks, the fairy's wall bombed, the lullaby, the magic; out, the owl flies Link to Kakariko |

The scope rule as before: every actor whole, except those a child's init kills (their init's
checks, then a placeholder).

## The user's calls

2026-10-15: the user took the recommendations on every call:
1. **The split and its order** as proposed: six milestones, the critical path first, the exit at
   milestone 4, the breadth (the rest of the cavern, the summit) after.
2. **The adult's parts:** `En_Go2`'s adult types (Link the Goron, Biggoron, the Fire Temple's
   Gorons) and `Demo_Du`'s other cutscenes (the Fire Medallion, the Chamber after Ganon, the
   credits) are placeholders chosen at init, as `En_In`'s race and `Demo_Im`'s other types were;
   they come whole with Phases 15 and 19. `En_Gm` (350 lines) is ported whole, the knife's sale
   included.
3. **`En_M_Thunder`** comes with the magic meter in milestone 6.
4. **`EffectBlure`** is ported with `En_Zf`, and its existing callers (Link's sword, the Skulltula)
   are wired to it then.
5. **The coverage table's corrections** are applied with milestone 1's coverage refresh.

The scope rule stays: every actor whole, except those a child's init kills, and the adult's parts
above.

## Milestone 1: Bombs

**The answer:** at Dodongo's Cavern's entrance with the Goron's Bracelet and the 20 bomb bag (a
debug start, `cavern-bombs`), child Link lifts the west Bomb Flower, carries it to the entrance
wall, puts it down and walks off: its blast breaks the wall, and the cavern's opening plays
(`gDcOpeningCs`, the title card, `EVENTCHKINF_B0`). Then a bomb out of the bag (C-Right), put down
by the east flower: its blast sets the flower off. Scripts: `scripts\run\test-bombs.bat`,
`game-bombs.bat`, `sandbox-bombs.bat` (menu 132 to 134).

### What was built

- **`En_Bom`** whole (ADR 0072): its fuse (70 frames, appearing at 67, sparks and smoke under 63,
  the flash quickening at 40, 20 and 3), held, put down, thrown (the wall's rebound, the bounce,
  offered again on the ground), set off by a hit or an enemy's touch, put out by deep water; the
  blast (`Effect_Ss_Bomb2` layered, the shockwave, the light flash, the quake) and ten frames of a
  `DMG_EXPLOSIVE` sphere growing by 8 and `shape.rot.z`'s low byte, its damage raised by the high
  byte; letting go of Link when it goes in his hand. Two bakes.
- **`En_Bombf`** whole: the flower (offering its bomb, the hand-over to the bomb it spawns when
  lifted, a hit's bomb that blows at once but from a boss, a burning stick's with 100 on the fuse,
  the regrowth, letting go of Link), and its bomb (a fuse of 140, the green flash quickening at
  70, 50, 30 and 3, its own blast and sphere); its wall and ceiling flowers' tilt in the schema.
  Two bakes (the bomb's spark on segment 8's billboarded matrix).
- **`Effect_Ss_G_Spk`** whole: the fuse's sparks following their explosive (and its unused update
  mode and spawners).
- **Player:** `Player_InitExplosiveIA` (the bomb spawned in hand through a predicted handle,
  `PlayRequest::SpawnHeldExplosive`), the bombs' upper body (`Player_UpperAction_CarryActor`);
  `PlayerIface::held_actor`, `hand_over_held_actor` (`func_8002F5C4`, in `oot_game::actor_ctx`),
  `clear_carried`; `Actor_OfferGetItem`'s `Player_GetExplosiveHeld` check;
  `ActorContext::next_handle`.
- **The callers:** the Peahat's death spawns a bomb with no fuse, the courtyard guard throws a
  bomb (BACKLOG #33 closed).
- **The `cavern-bombs` preset** (`ranch-epona`, the bracelet, the 20 bomb bag on C-Right), and
  **`Route::Bombs`** with the tasks `LiftBombFlower`, `PutDown`, `TakeBomb` and the steps
  `FlowerLifted`, `PutDown`, `DcOpening`, `BombOut`, `FlowerSetOff`.
- **Coverage:** the `SCENES` and `ACTORS` corrections (the user's call); Phase 10 in progress, 33
  actors placed (with `En_Fire_Rock`), 2 ported.

### Results

- `tests/bombs.rs` (8): a bomb taken out in hand, its fuse frame by frame from the C (69 at the end
  of its spawn's frame, the scale at 67, the sparks under 63, the flash's halvings, 0.014 at 1),
  its blast in hand (the light 250 then 225, Bomb2 and the shockwave, the sphere 8 then 72, the
  release, the hit on Link, the bombs put away); a bomb put down and offered again; the entrance
  wall's opening from a blast; the lobby's boulders (flag 0x15) and the flower between them; the
  flower's tug without the bracelet; its lift, the hand-over and the regrowth (180 frames, then 20
  steps of 0.05); the spark following its bomb; the exit's run. `tests/field.rs`'s Peahat test goes
  on through its bomb (damage 14, growth 2: the sphere 10 a frame).
- The exit's run: 572 frames. The flower lifted at 105, put down at 190, the opening over at 425;
  the bomb out at 494, put down at 508, the east flower set off at 572. One bomb spent, no hit.
- 1,115 tests (one ignored) pass; `cargo test -p layering` passes; no warnings.
- Goldens: `field`'s trace changed (proven below), 209 the same; 5 new cases (`bombs`,
  `bombs_carry`, `bombs_blast`, `bombs_opening`, `bombs_bomb`). 216 hashes, 166 cases
  (`out/golden_base23`).

### Decisions

- Player's explosive in hand goes through a predicted handle (ADR 0072): the rest of his update
  reads `heldActor` as the C's does, and the bomb still updates the same frame after him.
- The explosion's sphere is centred where the bomb last drew, as the C's: `Collider_UpdateSpheres`
  in `draw_update`.
- The run is a debug start at the cavern's entrance (the user's split put the bracelet in milestone
  2 and the bag in 3).

### Corrections to the survey

- The proposal's run ("a debug start with a bomb bag breaks a wall, a rock and a crate"): from the
  cavern's entrance the lobby's other walls, its boulders and crates are past the pit (a search over
  the collision finds no walk to the side wings: milestone 3's pillars). The run uses the
  entrance's two flowers instead; the lobby's boulders are a test. The Peahat's death is a test too,
  and `Route::Field` already killed one (its trace changed).
- The cavern's wall and ceiling flowers are tilted by `rot.x` (0x4000, 0x8000): the schema names
  it.
- The survey's "no `PlayerIface` write" for an explosive's release is three methods; the hand-over
  needed Player's `heldActor` from outside his update too.

### Known gaps

- Bombchus (`En_Bom_Chu`, BACKLOG #70); the rumble (#60); the circle shadows and the bomb's shine
  (`func_8002EBCC`: the rendering debt).
- A soft stick up the cavern's entrance ramps leaves Link standing (#71: to check by hand).
- Link's bracelet isn't drawn on him (milestone 2); the cavern's draw config (its lava, the
  skull's eyes) is milestone 3's: the lobby's materials are their import-time values.

### Fixes found while building

- `PlayerIface::holds_actor` was always false: an item offered while Link carried something was
  refused (`Actor_OfferGetItem`'s `heldActor != NULL`).
- An offer straight in front of Link (a yaw difference of 0x8000) was refused: the C's
  `ABS(yawDiff)` is 32768 in an `s32`, the port's was -32768 in an `i16`.
- Stale docs: `Bg_Ydan_Sp` (Link holds a Deku Stick since GAME-05), `interface.rs` (the timers are
  ported), `Bg_Breakwall`, `Obj_Bombiwa`, `Obj_Kibako2`, `En_Wf`, `En_Peehat`, `En_Heishi2` (bombs).
- `scripts/__pycache__` is ignored (a `.pyc` had been committed with the exit review).

### Golden proof

- `field`'s trace: only `actors` differs, from frame 398 (the Peahat's death) to the end, one fewer:
  the old placeholder bomb stayed, the real one explodes and is gone ten frames on. **A**, this
  code with `En_Bom` unregistered (its spawn a placeholder again, `target/game26proof`): all six
  `field` hashes back (`out/golden_proof_g9m1_A.txt`).

### How to check

```
scripts\run\test-bombs.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-bombs.bat
```

### By hand

1. `game-bombs`: Dodongo's Cavern's entrance. Walk west up the ramp to the Bomb Flower on the
   raised strip; A by it: Link lifts its bomb (it fizzes, a spark at its top, flashing red as the
   fuse runs down) and the flower grows a new one. Carry it to the wall ahead, stand still, A: it's
   put down. Walk away: it explodes, the wall breaks, the camera looks up into the cavern
   ("Dodongo's Cavern").
2. C-Right (L): a bomb in Link's hands (19 left). A or L standing: put down; while running: thrown.
   It flashes faster and explodes about two seconds after it came out.
3. Hold a bomb until it explodes: half a heart off, Link knocked about, his hands empty.
4. A bomb put down next to the east flower: its blast sets the flower off a moment later.
5. Pick a lying bomb up again (A by it) before it explodes.
6. Walk slowly (Shift) up a ramp to a flower strip: does Link stop halfway? (#71)
7. `game-bombs field`: Hyrule Field by day. Kill a Peahat (Z and slash it when it lands, or its
   root): it shrinks rising and explodes, three drops.

## Milestone 2: The trail and Goron City to the bracelet, and the cavern's mouth

**The answer:** in one step (the user asked for the milestone whole with one hand test; the
cavern's mouth is small and needs the city's bracelet anyway). Two short scripted runs from
debug starts, no enemies on the way (the user's standing preference, re-stated during the
milestone): **Darunia** (`Route::Darunia`, from Goron City's bottom floor on `ranch-epona`):
Zelda's Lullaby at the spot by his door, the door rising, Saria's Song to him, his dance
(`gGoronCityDaruniaCorrectSongCs`) and the Goron's Bracelet; **the boulder** (`Route::Boulder`,
from the cliff above Dodongo's Cavern on `goron-bracelet`): the cliff's Bomb Flower lifted and
thrown at a run off the ledge, the boulder's watch starting one-point 4180 as it falls, its blast
breaking the boulder (switch 4, `EVENTCHKINF_23`). The whole chain (Kakariko's gate, the trail's
Tektites, the city, back up to the cliff, into the cavern) is the hand test's. Scripts:
`scripts\run\test-goron.bat`, `game-goron.bat`, `sandbox-goron.bat` (menu 135 to 137).

### What was built

- **`En_Go2`** (ADR 0073): every child Goron (the trail's 4, 5, 6 and 0xC, the city's 0 and 7 to
  0xB) and the bazaar's 0xD, whole: texts and flags, waking and curling, the dust, the rolling
  states (the trail's roller and his bomb at his path's end; the city's big roller stopped by a
  blast, his 30 or 40 bomb bag), the get item, the draw (eyes and mouth, curled and rolling
  lists). The adult's types 1, 2 and 3 are placeholders chosen in its init (the user's call), after
  the init's own kills. 16 bakes.
- **`En_Du`** whole: his talk (0x301A, 0x301B, `INFTABLE_113`; 0x301D with the bracelet, 0x301E
  with the Ruby), the ocarina asked for within 136, Saria's Song's and another song's cutscenes
  started in place, his dance on cue channel 2, 0x301C and `Actor_OfferGetItem(GI_GORONS_BRACELET)`,
  the layer-4 dance, the Fire Temple's meeting (one-point 3330 logs). `Bg_Spot18_Shutter` (the
  child's rising door on switch 0x1B, the adult's sliding one with one-point 4221) and
  `Bg_Spot18_Obj` (the statue and the child's spear; the adult's pull) whole.
- **`En_Tite`** whole (red and blue: the blue's water walking; the ripples log); `En_Part`'s
  Tektite pieces with their textures; `En_Encount1`'s Tektites real.
- **`Bg_Spot16_Bombstone`** (the boulder, its debris, the push, the watch of a carried Bomb
  Flower, 4180), **`Bg_Bombwall`** (the trail's three walls), **`En_Hata`** (the flag in the
  room's wind) whole.
- **`Bg_Spot18_Basket`** and **`Bg_Spot18_Futa`** (the urn: 4220 on the torches' switch 0x1C,
  4210 for a bomb in its mouth, its three prizes), **`En_Gm`** (Medigoron, the knife's sale
  included; his animation decoded on `gGoronSkel`, `foreign_anims`), **`Obj_Hamishi`**,
  **`Obj_Hsblock`** whole.
- **Systems:** the room's wind command (`RoomData::wind`, pack format 29, `envCtx.windSpeed`);
  one-point cutscenes 4180, 4210, 4220, 4221; `SDC_GORON_CITY`; the child's bracelet drawn on
  Link (`BakeSegment::SkeletonMatrices`); Player's idle fidget from the room's environment type
  (Goron City's rooms are `ROOM_ENV_UNK_STRETCH_1`).
- **The runs:** `Route::Darunia`, `Route::Boulder`, `Task::ThrowAtRun` (a carried actor thrown at
  a run from a point), `Cond::DaruniaDoorOpen`; the presets **`goron-bracelet`** (the Darunia
  run's end: the door's switch, the bracelet, and the two intros) and **`cavern-open`** (the
  boulder run's end: milestone 3's start), each checked against its run.
- **The characters** (`export-glb`): Goron, Darunia, Medigoron, Red Tektite, Blue Tektite.
- Five helpers ported the overlays in worktrees (removed); BACKLOG #34's Tektites and Wolfos parts
  struck (the Leevers remain).

### Results

- Tests from the C: `tests/en_go2.rs` (12), `en_du.rs` (13), `en_tite.rs` (12),
  `bg_spot16_bombstone.rs` (4), `bg_bombwall.rs` (3), `en_hata.rs` (2), `bg_spot18_basket.rs` (10),
  `en_gm.rs` (6), `obj_hamishi.rs` (6), `obj_hsblock.rs` (4), `goron_city.rs` (2: the runs);
  `oot_game`'s scene tests (the wind, Goron City's entrance textures), the import's draw-config
  check (`SDC_GORON_CITY`).
- The runs: Darunia 2,609 frames (the ocarina turned to the song spot at 81, the lullaby at 117,
  the door open at 399, Saria's Song at 743, the bracelet's text read at 2,609); the boulder 258
  frames (the flower lifted at 57, thrown at 97, the boulder blown and the cameras back at 258). No
  hit taken in either.
- Goldens: 208 of milestone 1's 216 the same; 8 differ (proven below): the bracelet on child Link
  in 7 images, the fidget in the ranch's trace. 8 new cases (`darunia`, `darunia_door`,
  `darunia_dance`, `boulder`, `boulder_throw`, `boulder_blast`). 224 hashes, 172 cases
  (`out/golden_base24`), the same over two runs. After the hand test's fix (below), 34
  re-recorded: `out/golden_base25`.
- 1,200 tests (one ignored) pass; `cargo test -p layering` passes; no warnings. The pack
  is format 29 (`out/data26`; `scripts/run/_env.bat` points there).

### Decisions

- One step, two short runs from debug starts (above); the story chain is hand-tested.
- The adult's Gorons are placeholders chosen in `En_Go2`'s init; `En_Du`'s, `Bg_Spot18_Shutter`'s
  and `Bg_Spot18_Obj`'s adult parts and `En_Gm`'s sale are ported whole (the user's calls).
- The bracelet's list follows Link's bones through the skeleton's matrix order baked into a new
  segment kind rather than a Link variant (it's drawn after the skeleton, as in the C).
- `windSpeed` stays 0 in a room without the wind command (`Environment_Init` doesn't set it:
  `@bug (game)`).

### The questions the survey left open

- **The wrong song's text 0x3020 doesn't set `EVENTCHKINF_22`:** Darunia's talk state stays idle
  through the cutscene, so `Npc_UpdateTalking` never runs `EnDu_UpdateTalkState`; the script's
  0x3020 is a cutscene text, and his `textId` stays 0x301A or 0x301B. (`z_actor.c`'s older text
  handler sets it for 0x3020, but `En_Du` doesn't use it.) A test checks it.
- **Goron City's Lost Woods passage** (to exit 4, its floor north of z 1240) is sealed by all three
  boulders together: `Obj_Bombiwa` 0x0008 (367, 196, 1078), 0x800C (416, 196, 1049, the chime)
  and 0x000B (470, 196, 1031), cylinders of radius 55 across a passage about 160 wide there (a
  walk north stops at z 982 to 1024). The Bomb Flowers in front, (371, 196, 1020) and (438, 196,
  999), and the wall ones at (339, 258, 1057) and (491, 256, 1003), are what blow them.
- **The bomb-flower Goron's walk** (type 4, path 0): after 0x300B he aims for his own first point
  (`EnGo2_UpdateWaypoint` wraps at `count - 1`), walks two frames (0.25 units) and sits again; with
  `INFTABLE_EB` his next init puts him at the path's last point, (-1263, 1412, -730). Neither spot
  covers the cliff's Bomb Flower (152 and 158 from it).
- **No hot rooms for a child:** the trail's room is `ROOM_ENV_DEFAULT`; Goron City's four rooms are
  `ROOM_ENV_UNK_STRETCH_1` (Link's stretching fidgets, ported with this milestone).
- **Entrances** checked against `ootx scene-info` and the entrance table: the trail's 0 (the
  intro), 1 (Goron City's door), 3 (before the cavern); the city's 0 (its intro), 2 (the bottom,
  no intro), 3 (the Lost Woods).

### Corrections to the survey

- The trail's rolling Goron stops at his path's second-to-last point (7), not its end: the
  waypoint wraps at `count - 1`; his bomb blows at about (-1526, 1056, 1224), 255 frames after Link
  comes within 1000. He and the other Gorons of types 3 to 0xD are update-culled: he rolls only
  while the camera sees him. Rolling speeds are 1.5 times their value a frame (`Actor_UpdatePos`).
- The big rolling Goron's 6 after his gift eases back to 3.6 (`EnGo2_SlowRolling`).
- The Gorons' first texts run on into their repeat texts in one box (the message data's next-text
  codes): 0x3008 to 0x302A, 0x3014 to 0x3015, and so on; the flags key on the first.
- Medigoron's 0x3049 is effectively never seen: his talk reach is past the inner wall, whose
  breaking sets his flag 6. A child reaches room 2 only with explosives (four walls on the way,
  flags 2, 4, 5, 6).
- The urn's torches are lit from Darunia's room's torches (the y-196 floor's two start unlit
  unless switch flag 0 is set); the urn's bomb prize drops nothing without a bomb bag.
- `Bg_Bombwall` breaks two updates after the hit; the boulder's watch is a fixed area of the world
  (400 round (-1579, -790), below y 1400), and a thrown flower from x -1413 enters it 17 frames on.
- `En_Hata`'s placements: the trail and spot11/12/13; the trail's wind is (-100, -1, 20) at 130.

### The hand test

- The user saw the blue attention arrow over Darunia through his dance. Not the game's: the C's
  `Attention_FindActor` searches nothing while `Player_InCsMode` (`z_actor.c`), so in a cutscene
  or a talk there's no arrow and Navi hovers over no actor (but a locked one). The port searched
  on. Fixed in `oot_game::target::update` (and `bgmEnemy` is left as it was then, as in the C);
  `tests/goron_city.rs` checks the dance (an arrow over him before it, none in it), and fails
  without the fix (frame 868). Pre-existing since the attention arrows were ported: every
  cutscene with something targetable in reach had it.
- Its knock-on: Navi flies differently in cutscenes and talks, and her `Rand` calls move the
  random sequence after. Talon's house's tests talked to him with a cucco wandering into Link's
  lift reach, which an A lifts before a talk: they now wait for none in reach (the test's
  luck, not the game).
- Found on the way, logged (#79): the C searches the boss, enemy and Bg categories before the
  others (`sAttentionCategorySearchOrder`); the port takes the nearest of all.

### Known gaps

- The adult's Gorons (#72), Darunia's 3330 (#73), the blue Tektite's ripples (#74),
  `Obj_Ice_Poly` (#75), the hammer's floor hit (#76), an offered item taken a frame late after a
  talk (#77), mask reactions (#78).
- The summit (`En_Encount2`, `En_Fire_Rock`, the fairy) is milestone 6's; the cavern milestone 3's.
- Player's idle fidget is still the port's reduced form for the common types (only the open-hand
  ones; the critical-health fidgets aren't chosen).

### Fixes found while building

- Player's idle fidget ignored the room's environment type (always the default room's).
- A Goron test set the torches' switch and then talked at once: the urn's 4220 holds Link (the
  C's behaviour); the test waits it out.

### Golden proof

- The bracelet: `bombs`, `bombs_carry`, `bombs_bomb` (shots), `course_run-roll_child`,
  `course_sword_child`, `spot04_child_forward`, `spot04_tread` (sheets) differ by a few hundred
  pixels on child Link's left forearm: `cavern-bombs` gives the bracelet, and the spikes' debug
  save's upgrades (0x125249, `Sram_InitDebugSave`'s) hold strength 1, which `Player_DrawImpl` draws.
- The fidget: `ranch`'s trace differs only in `anim` and `root_joint`, frames 2680 to 3073 (Link
  waiting for the foal): `link_waitF_typeD_20f` by the ranch's room environment, where the default
  room's `link_normal_waitF_typeA_20f` was.
- **A**, this code with the bracelet's draw off (`target/game26proof`): every changed hash back but
  the ranch's trace. **B**, with the fidget's room environment 0: the ranch's trace back, the seven
  images still changed (`out/golden_proof_g9m2.txt`).
- The hand test's fix (no attention search in a cutscene mode): 34 hashes of base24 differ, each
  explained in `golden/README.md`. **C**, this code with the search back in cutscene mode: all 224
  of base24's hashes back (`out/golden_proof_g9m2_C.txt`).

### How to check

```
scripts\run\test-goron.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-goron.bat
```

### By hand

1. `game-goron` (the trail's foot, `ranch-epona`): the trail's intro (the title card). Walk up the
   west side: four red Tektites crouch, hop at Link and lunge; two sword hits kill one (it falls to
   pieces, a drop). Past the cavern's terrace, up the switchback east of it and on up the road:
   the rolling Goron comes down the road once he's on screen (touching him: a quarter heart and a
   knock-down), stops near the bend, a bomb goes off where he stops, and he curls up; wake him
   (walk up in front) and talk. The flag on the way streams west in the wind. At the top, Goron
   City's door (north).
2. Goron City (from 1, or `game-goron city`): its intro. The Gorons sit curled; in front of one
   within reach he uncurls; A to talk; walk away and he curls again. The big one rolls round the
   middle ring (y 400). The city's walls are lit as by day (the entrance's light at the top).
3. `game-goron darunia` (the bottom floor): south to the floor before Darunia's door (by the
   text spot), C-Down for the ocarina, Zelda's Lullaby: the door rises (about 6 seconds). In, up
   to Darunia: he asks for a song. Saria's Song: the music changes, he dances, then gives the
   Goron's Bracelet. Look at Link's left forearm: the bracelet. Talk to him again (0x301D). Also:
   another song to him first (the lullaby): a short "wrong" cutscene, and he waits again.
4. `game-goron cliff` (by Goron City's door with the bracelet): north-west up the road to the
   cliff above the cavern (the Goron sitting there talks about the flower). A by the Bomb Flower
   lifts it; run west off the ledge's edge and A: thrown. The camera cuts to the cavern's mouth
   (a fixed shot) as it falls; the blast breaks the boulder, pieces and dust, a chime. Then down
   the road to the cavern (or `game-goron cavern`) and in.
5. `game-goron bombs` (Goron City's top with bombs): a bomb (C-Right) put in the big rolling
   Goron's way on the middle ring: he stops; talk (between the city's north-east and east points
   he gives the 30 bomb bag). The two walls on the top floor's west corridor and two in the
   room behind: bombs break them; Medigoron talks. In the bottom floor, a bomb dropped into the
   urn's mouth (from the ring above) spins it wild and it gives a prize (only after the four
   torches are lit with a burning Deku Stick from Darunia's room: hard; skip if so).
6. Anything that looks off: the Gorons' or Darunia's faces, the Tektites' colours, the flag, the
   urn, the bracelet's look on Link. Also #71 (a slow walk up a ramp) on the trail's slopes.
