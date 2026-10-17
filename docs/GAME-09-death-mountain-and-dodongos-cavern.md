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
| 3 | Dodongo's Cavern's route | done (2026-10-16) |
| 4 | King Dodongo and the Goron's Ruby (the phase's exit) | done (2026-10-17): the exit met |
| 5 | The rest of the cavern | next |
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

## Milestone 3: Dodongo's Cavern's route (Master Quest)

**The answer:** route B, the loop to the skull's bridge (the user's call), as the search over the
collision found it. Four short scripted runs from debug starts of its moments (no fights: those
are the hand test's, the user's standing rule): **the shortcut pillar** (`Route::Pillar`, the lobby's island: the island's
Bomb Flower thrown at the pit's boulder, the switch, one-point 3060, the ladder, the pillar to its
top and the ledge grab onto the upper level), **the bomb bag** (`Route::BombBag`, the east
corridor on `cavern-loop`), **the skull** (`Route::Skull`, the bridge on `cavern-bomb-bag`: bombs
onto C-Right in the pause menu, one put down through each hole, the eyes, 3065, the jaw, 3380) and
**the gravestone** (`Route::Gravestone`, room 7's alcove on `cavern-jaw`: the gravestone pulled
off switch 0x20, door 20, the fall to King Dodongo's room). The whole loop is the hand test's.
From `cavern-open` on, Link wears the Hylian Shield (the user's note: it matters in the cavern): the
graveyard's shield grave's chest (`hakaana2`'s `En_Box` 0x5540, `GI_SHIELD_HYLIAN`), ported in
GAME-07 but never opened by the story chain's presets.
Scripts: `scripts\run\test-cavern.bat`, `game-cavern.bat`, `sandbox-cavern.bat` (menu 138 to
140). **One open question for the hand test:** how a child brings fire to room 10's web (below).

### The way a child goes (settled at the milestone's start)

The survey left the climbs open (`ootx` dumps no collision). A search over the cavern's whole
static collision settled them: a 25 grid, every floor of each cell (`BgCheck_EntityRaycastDown`
from above, again and again under each one), a step to the next cell's highest floor at or under
71 above (Player's ledge raycast from `pos.y + unk_0C`, `sAgeProperties[child]` 71): walked under
12 (17 diagonally), climbed from 18 to 71 against a wall onto a floor flatter than `0x6D60`,
fallen otherwise; walls tested 20 and 45 up; the ladders (wall type 2, `WALL_FLAG_1`) joined
bottom to top; run-off jumps and mid-air ledge grabs; the pillars (`Bg_Ddan_Jd`,
`gDodongoRisingPlatformCol` at scale 0.1: ±60, its top 40 up) as dyna at their middle and top.
The jumps were checked with the port's Player on stand-in pillars.

- **The bomb bag's corridor (y 80) is 80 above the lobby's floor**, a sheer step at x 800: a
  child's ledge check reaches 71, an adult's 111. From the lobby it's a way down, not up; a child
  reaches it from room 4, through door 22 (plain from room 0's side, barred from room 4's until
  0x1B).
- **Room 3's two levels aren't joined for a child.** The lower level is lava at y 40 (floor type
  2, Player's hot floor) with eight platforms at 100 (`sDCPlatformPositions`' lower floor); the
  upper is lava at 471 with platforms at 531. No ladder, no climb between them. And a paired
  Lizalfos kills itself at init when Link is more than 100 above or below it
  (`z_en_zf.c:411-419`): come in at the top, the lower pair is gone, and doors 8 and 9 stay
  barred (`SHUTTER_FRONT_SWITCH` on 0x05 from room 3's side, `DoorShutter_SetupDoor`): Link would
  be shut in below. So the lower pair is fought from below (room 1's door 9) and the upper pair
  from above (room 10's door 10).
- **The lobby:** the pit is lava (y -180, floor type 2) with a -160 rim; two ladders (x ±636,
  z -1662) climb from it to the side floors (y 0); the three pillars at their middle height are
  level with the floor, the gaps round them 120 to 130 (a child's run-off jump covers about 140:
  tested). **The only way up to the upper level (y 531) is the shortcut pillar** 0x000A (route
  A's pillar is route B's way up too): the island's Bomb Flower (-2, 0, -1170) carried off the
  island's south-east corner and thrown at boulder 0x000E on the -160 shelf, the boulder blown off
  switch 0x0A; the pillar's top (560 at its top) is 170 short of the upper level (x 650), and
  Player's mid-air ledge grab takes Link up when he runs off it at z -1150 to -1170 (south of
  that the upper level's face slants, z -1190 to -1250, and the grab slides off).
- **Room 1's wall 0xA01F (820, 0, -320) can't be reached with a flower from the floor:** about
  1,400 units from the island's flower by the pillars, against its 140-frame fuse. The upper
  level's Bomb Flower (850, 531, -923) can: carried to the opening in the upper level at z -725 to
  -500 (x 625 to 800) and dropped 531 (a half heart: `func_80843E64`'s impact 0), about 400 from the
  wall. But its three boulders 0x0015 (radius 55, `z_obj_bombiwa.c:61`) seal the strip it stands
  in (z -1000 to -850 past x 800, 150 wide), the flower just behind the west one: it's reached from
  the strip's east end, through room 9 (in by `En_Holl` 5 from room 12, a run-off jump west over
  room 9's 110 gap at x 1725 to 1835, out by `En_Holl` 2).
- **Room 9's east ledge** (531, `En_Holl` 6 to room 10) is up a ladder (x 1996, z -320) whose
  foot is at 411, 60 above the trench (351): a push block (`Obj_Oshihiki` 0xFF40, 60 tall) must
  stand at its foot. Block 2 (1740, 351, -450), pushed east four times and north twice, does.
- **Room 10's web** (`Bg_Ydan_Sp` 0x1FD4) takes only fire (`DMG_ARROW_FIRE`, `DMG_MAGIC_FIRE`, a
  burning stick within reach). See the open question below.
- **Room 7's switch alcove** (y 126, the gravestone over switch 0x20) is 110 above room 7's floor:
  a corridor north and then east to `En_Holl` 4, entered from room 8.

The way, then: the lobby's pillar to the upper level → room 9 → room 10 (the web) → room 3's
upper pair → room 12's fire walls → room 9 → the strip's flower, the boulders, the flower again,
dropped by wall 0xA01F → room 1 (the eye switch, its boulder 0x8012 under the flower cluster, the
slingshot) → room 3's lower pair → room 4 (the Poes, the chain to `En_Wonder_Item` 0x18DB) → door 22
and **the bomb bag** → down to the lobby, the pillar (shuttling now) back up → the bridge, bombs
into its holes, the eyes and the jaw → room 7 → room 8 (Armos, a Lizalfos, fire walls, clear
blocks) → room 7's alcove, the gravestone, switch 0x20, door 20 → room 16's pit. Room 1 and room 4
both: so `En_Poh`, `En_Am`, `Bg_Gnd_Darkmeiro` and `Obj_Elevator` came into this milestone from
milestone 5.

### The open question: fire for room 10's web

Found while writing the hand test, after the build: **I found no way for a child to bring a
burning Deku Stick to room 10's web.** The sticks are the lobby's (its west Business Scrub,
`En_Shopnuts` params 1); the only torch that burns from the start on that side is room 9's wooden
one on its west ledge (`Obj_Syokudai` 0x2400, `alwaysLit`). From there the way to room 10's door is
down into the trench and up the east ladder, and a ladder grab puts a held item away
(`func_8083EC18`: `Player_SetupWaitForPutAway`, which `Player_PutAwayHeldItem` does for every
item action from `PLAYER_IA_FISHING_POLE` up, the stick's 6 among them); taken out again, the
stick is unlit (`Player_InitItemAction`: `unk_860` 0). A climb of 47 or more (the auto-jump and
the ledge grab, `Player_Action_80845668`) doesn't put it away, but no such climb reaches the east
ledge (the block's top is 120 below it). Room 10's torch before the web (0x10A8 at (2556, 531,
-370)) is a timed one, unlit at the start; room 9's three timed torches and its golden one are on
the trench's west side and its pedestal. Nothing else in the scene sets the web's flag 0x14, and a
Bomb Flower behind the strip's boulders (the other way round, by `En_Holl` 2) takes explosions,
arrows and fire only (`0x0003F828`), not a slingshot seed.

So either the game has a way I haven't seen (the user knows the real game), or route B isn't a
child's without something more. For the hand test: try it (`game-cavern` from the start, or
`game-cavern upper` on the upper level); if the web stops you, `game-cavern web` starts in room 10
past it (`cavern-web`, a debug preset with its flag set), and the rest of the loop goes on from
there.

### What was built

- **`Bg_Dodoago`** (the skull): the eyes lit by blasts on either side (`Actor_GetCollidedExplosive`,
  the yaw to the blast), one-point 3065, the glow in `roomCtx.drawParams` up 5 a frame, the jaw
  (3380, the dust on `sTimer` 108, the sparkles, `gDodongoLowerJawCol`), the bomb catcher under the
  bridge's holes (an `En_Bom` of params 0 held at fuse 50), the statics freed with the overlay.
- **`Bg_Ddan_Jd`** (the pillars): idle and rising on their timers, the shortcut on its switch
  (3060), the flag's shift masked to 5 bits as the console's (0x00FF reads flag 0x1F). **`En_Dodojr`**
  (the Baby Dodongos): buried, surfacing within reach, the crawl, the jump at Link, eating a bomb,
  the stun, the death's swell and blast. **`Obj_Elevator`** (room 1's lifts).
- **`En_Zf`** (the Lizalfos and the Dinolfos): every type, the pairs' init kills and turns
  (`sPairedActiveFleeFlag`, `sPairedInactiveType`), the platforms (`sDCPlatformPositions`), the
  mini-boss music, the fighting, the damage table, the sword's trail (`EFFECT_BLURE1`), eight bakes.
- **`En_Poh`** (the Poes and the composers Sharp and Flat), **`En_Am`** (the Armos: the statue, the
  enemy, the death's bomb), **`Bg_Hidan_Curtain`** (the fire walls), **`Bg_Gnd_Darkmeiro`** (the
  clear blocks and their switches), whole.
- **`EffectBlure`** (`oot_game::effect::blure`, ADR 0074): both inits, the calc modes, the vertices
  and spaces, the update, the three draws (the strip, the simple quads, the smooth Hermite strip)
  through per-vertex matrices on a bake's segment 0x0C (`BakeSegment::Matrices`). Wired to Player's
  sword (`D_8085470C`), `En_Zf`, `En_St`, `En_Ssh` and `En_Arrow`'s four arrows.
- **`SDC_DODONGOS_CAVERN`** (the scrolls, the entrance's texture by night, the lava floor's eight
  frames, the eyes' glow) and one-points **3060, 3065, 3380**.
- **The runs:** `Route::Pillar`, `BombBag`, `Skull`, `Gravestone`; the tasks `Task::PullStone`, `Task::RunOntoPillar`, `Cond::Switch`, `Cond::Pillar`, and `Task::ClimbTo`
  holding the stick along Link's facing while he hangs (`Player_Action_8084BBE4` climbs on it).
- **The presets:** `cavern-lobby` and `cavern-loop` (debug starts), `cavern-upper` (the pillar run's
  end), `cavern-bomb-bag` (the bag run's end), `cavern-jaw` (the skull run's end: bombs on
  C-Right, 18 left, flag 0x1A), `cavern-boss` (the gravestone run's end: milestone 4's start),
  each of the four checked against its run; `cavern-web` (room 10's web burned, for the hand test).
- **The characters** (`export-glb`): Lizalfos, Dinolfos, Poe, Poe Composer (Sharp), Poe Composer
  (Flat), Armos, Baby Dodongo (checked in Blender).
- Helpers ported the overlays in worktrees (removed).

### Results

- Tests from the C: `tests/bg_dodoago.rs` (10), `bg_ddan_jd.rs` (6), `en_dodojr.rs` (6),
  `en_zf.rs` (13), `en_poh.rs` (15), `en_am.rs` (8), `obj_elevator.rs` (3), `bg_hidan_curtain.rs`
  (5), `bg_gnd_darkmeiro.rs` (5), `cavern.rs` (4: the runs); the trails in `sword.rs` and
  `skulltula_st.rs`, the keyframes in `onepoint.rs`; `oot_game`'s scene tests (the draw config,
  the eyes' glow) and `EffectBlure`'s own (4); the import's draw-config check.
- The runs (frames from their debug starts): the pillar 819 (the flower lifted at 61,
  thrown at 181 from the lava the carried run-off jump falls short into: a quarter heart; the
  boulder at 198, the switch at 316, the ladder at 433, onto the pillar at 582, the upper level at
  819); the bag 358; the skull 470 (the menu 52 to 74, the bombs put down at 108 and 249, the eyes
  at 217 and 470: the first bomb caught, fuse 50; the second not, the skull still holding the
  first blast as its parent; no hit); the gravestone 447 (pulled at 125, the switch at 222, door 20
  at 385, the fall at 447; a Baby Dodongo by the door: half a heart).
- Goldens: 217 of milestone 2's 224 the same; 7 differ (proven below): two traces' route input
  on a hanging frame, and the cavern's entrance cases (the lobby's pillars' `Rand` calls, the
  skull's jaw and the draw config in the opening's view). 8 new cases (`cavern_pillar`,
  `cavern_pillar_ride`, `cavern_bomb_bag`, `cavern_skull`, `cavern_skull_jaw`, `cavern_gravestone`,
  `cavern_gravestone_pull`, `course_sword_trail`). 236 hashes, 180 cases (`out/golden_base26`),
  the same over two runs.
- 1,281 tests (one ignored) pass; `cargo test -p layering` passes; no warnings. The pack is still
  format 29 (`out/data26`).

### Decisions

- Route B by the user; within it the search's way (above). The runs are short debug starts; the
  loop, which no run walks whole, is the hand test's.
- The skull run puts the bombs on C-Right through the pause menu: the bag's get item equips
  nothing (`Item_Give`).
- `EffectBlure`'s bakes put each vertex at the origin under its own matrix, the draw setting the
  matrices' translations: the trail's vertices move every frame.
- No run fights: the Lizalfos pairs, the Poes and the Armos are the hand test's
  (`game-cavern lizalfos` starts by the lower pair). The pairs' fights are checked from the C in
  `tests/en_zf.rs`.
- The Hylian Shield from `cavern-open` on (above), as the graveyard's chest gives it; the story
  presets before it are as they were.
- The small helpers the new overlays share with older ones (`decr`, `actor_setup_xlu_dl`,
  `actor_free_overlay`) stay file-local, as the codebase keeps them.

### Corrections to the survey

- The bomb bag's chest is a child's from room 4 (door 22), not from the lobby (above).
- Route B's "room 4 or room 1 into room 3": both are on the loop, room 1 into room 3's lower level
  (door 9) and out to room 4 (door 8); room 3's upper level is entered from room 10 (door 10).
- Route A's pillar is route B's way up too: the lobby has no other way to the upper level.
- Beyond the survey's list, the loop meets `En_Poh`, `En_Am`, `Bg_Gnd_Darkmeiro` and
  `Obj_Elevator` (ported here, from milestone 5's list).
- The bomb catcher holds one bomb at a time: the first blast to light an eye becomes the skull's
  parent with `sTimer` 50 and is let go only past 140, so a second bomb dropped in the next 90
  frames falls on its own fuse.
- The jaw's dust under the mouth comes only while the skull holds a parent as it opens
  (`@bug (game)`, in the module).

### Known gaps

- Room 10's web (the open question above).
- The point lights don't shade what they light (#80), the Poes' lantern fragments draw nothing
  (#81), the Fire Temple's fire walls' one-points 3350 and 3360 (#82); the light's glow (#31), the
  rumble and the Lens of Truth (#60: the clear blocks' path is drawn as `Actor_DrawAll` draws a
  lens actor without it).
- Milestone 5's: `En_Dodongo`, `En_Vm`, `En_G_Switch`, `Bg_Ddan_Kd`, the effects `D_Fire`,
  `Dead_Dd`, `Dead_Ds`, one-point 3050, rooms 2, 5, 6, 13, 14 and 15. King Dodongo (milestone 4)
  is a placeholder in his room.
- #79 stays open: no NPC stands among the cavern's enemies.

### Fixes found while building

- Link on the course and in the sandbox had no sword trail: `Player_InitCommon`'s `Effect_Add`
  ran only in a scene's Player init, not in `new_play`'s (and the sandbox's void-out respawn keeps
  the trail's slot).
- The pillar run's ledge grab slid off: `Task::ClimbTo` steered at its target while Link hung; it
  holds the stick along his facing now, as the climb wants.
- Two helpers' tests leaned on a quiet `Rand` (the Armos's lunge, a fire wall's flag with a
  Lizalfos in reach): they read the state the frame before, and put Link out of reach.

### Golden proof

- `castle` and `graveyard`'s traces differ only in `input`, on 2 frames and 1, Link hanging on a
  climb (his motion the same): `Task::ClimbTo` holds the stick along his facing while he hangs.
- `bombs`' trace differs only in `actors`, one more for frames 249 to 273 (a piece of the entrance
  wall's rubble lives a frame longer), and `bombs_blast`, `bombs_bomb`, `bombs_carry` differ: the
  lobby's pillars (`Bg_Ddan_Jd`, real now) shuttle from the start and their dust's `Rand` calls
  move the sequence. `bombs_opening` shows the lobby: the skull's jaw and the scene's draw config.
- **A**, this code with the trails not drawn (`target/game26proof`): the same 7 differ, no other
  (no case caught a trail before `course_sword_trail`). **B**, with `ClimbTo`'s hang branch off:
  the two traces back. **C**, with `Bg_Ddan_Jd` unregistered: `bombs`' trace and three shots back.
  **D**, C and `Bg_Dodoago` unregistered and `SDC_DODONGOS_CAVERN` out of the tables: all 7 back
  (`out/golden_proof_g9m3_A.txt` to `_D.txt`).

### How to check

```
scripts\run\test-cavern.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-cavern.bat
```

### By hand

1. `game-cavern` (the cavern's entrance, `cavern-open`: the bracelet, no bombs): the west Bomb
   Flower to the entrance wall, its blast, the cavern's opening. In the lobby, Navi's word about
   the lava. Link wears the Hylian Shield. Buy Deku Sticks from the west Business Scrub (for room
   10, see 4).
2. The pit (or `game-cavern pillar`, on the island): lift the island's Bomb Flower, run off its
   south-east corner towards the boulder on the shelf: Link falls short into the lava (a quarter
   heart); throw at a run towards the boulder. Its blast breaks the boulder; walk onto the switch
   under it: the camera on the pillar (3060), which then rises and falls. Out of the pit by the
   south-east ladder, north along the floor's edge, onto the pillar when it's level with the floor
   (a run-off jump), ride it up, and at its top run east off it at its middle: Link grabs the upper
   level's edge and climbs.
3. The upper level (or `game-cavern upper`): east through room 9's door. Room 9: down into the
   trench, push the block in the middle of the trench east four times and north twice, under the
   east ladder; onto it, up the ladder, east to room 10's door.
4. Room 10's web needs fire: light a stick at room 9's lit torch (its west ledge, by the door you
   came in) and try to get it to the web. If you find a way, tell me what it was; if not,
   `game-cavern web` starts past it.
5. Room 10: Baby Dodongos surface and jump at Link; Skulltulas drop from the ceiling (their spin
   leaves a white trail). East to door 10: room 3's upper level, the upper Lizalfos pair drops in
   (the mini-boss music; one fights while the other hops round the platforms; twelve hits; their
   swords leave trails). Door 11 west to room 12: the fire walls; through to room 9's north side,
   the jump west over the gap, out to the lobby's upper strip.
6. The strip's Bomb Flower from its east end: its blast takes the three boulders; then carry the
   regrown one to the opening in the upper level and drop down with it (a half heart) and on to
   room 1's wall (east, by the door to room 1): the wall breaks.
7. Room 1 (the lifts, the eye switch for the slingshot, the boulder under the flowers), then door
   9 (or `game-cavern lizalfos`): room 3's lower pair. Out by door 8 to room 4: the Poes (four sword
   hits each; their souls and lanterns), the Bomb Flower chain, the hidden switch, door 22, the
   lobby's east corridor: the bomb bag's chest (or `game-cavern bag`).
8. Down to the lobby, up by the pillar, onto the bridge (or `game-cavern skull`): bombs onto a C
   button in the pause menu, one put down at each hole's edge: each eye lights red with its camera;
   the second opens the jaw (the dust, the sparkles, the camera on the skull).
9. In through the jaw (door 7), room 7, room 8: the Armos statues wake when touched (the Kokiri
   Sword bounces off; a Deku Nut, then the sword, or a bomb), a single Lizalfos drops in, the fire
   walls, the clear blocks; on to room 7's alcove (or `game-cavern gravestone`): pull the gravestone
   (A at it, back away), onto the switch, through door 20 and into the pit: King Dodongo's room
   (`game-cavern boss`: milestone 4's start; he isn't ported yet).
10. Anything that looks off: the trails (Link's, the Lizalfos's, the Skulltulas'), the skull's
    eyes, the lava's flicker and the scrolls, the Lizalfos, Poes, Armos and Baby Dodongos, the fire
    walls. Known: no point-light shading, no glows, no rumble, no lens.

## Milestone 4: King Dodongo and the Goron's Ruby (the phase's exit)

**The answer:** the Goron's Ruby is reached; **Phase 10's exit is met.** Two short scripted runs,
no fight (the user's rule: the fight is the hand test's): **the intro** (`Route::KingDodongo`, from
the room's spawn on `cavern-boss`): a bomb out on C-Right and put down on the cracked floor, its
blast breaking the floor, the drop through the hole, and King Dodongo's intro on his own sub camera
(Link held at the arena's north side, the camera following him down, King Dodongo walking in with
the camera tilted, his roar, the title card "Infernal Dinosaur King Dodongo", the fire boss's
music) to the fight's start; and **the Ruby** (`Route::Ruby`, from a debug start after his defeat on
`cavern-boss-beaten`): the heart container, the blue warp, the Goron's Ruby (`EVENTCHKINF_25`,
`Item_Give`), `gDMTGoronRubyCs` on Death Mountain Trail's layer 5 (Darunia falls into the cavern's
mouth, beats his chest, slaps Link, gives the Ruby; three Gorons drop in and walk off) to its
terminator, and the trail's spawn 5. Their ends are the presets `cavern-boss-fight` and
**`goron-ruby`** (milestones 5 and 6 start from it). Scripts: `scripts\run\test-kingdodongo.bat`,
`game-kingdodongo.bat`, `sandbox-kingdodongo.bat` (menu 141 to 143).

### Settled at the start (from the C)

- **The room texture he writes** (ADR 0075): the room RAM the game writes is play state
  (`oot_game::room::RoomRam`), by room and the region's offset in the room's file, its bytes copied
  from the pack where the writes start and dropped when a room load takes its buffer; the room's
  draws replace the written textures by their segment-3 source (`DrawParams::texture_images`,
  ADR 0051's pattern for objects). His overlay's two textures (`sLavaFloorLavaTex`,
  `sLavaFloorRockTex`) are his own state, read from the pack (an overlay's data comes with its
  first instance; he's the only one).
- **`Play_SetCameraAtEyeUp`: modelled, not logged.** It was small: `CAM_VIEW_UP` in
  `Camera_Update` (the up set takes the view's place once), `CamView::up`, and the view matrices
  (the play state's, the effects' billboard, the screen positions, the renderer's camera) using it.
  An up within 0.1° of the view's vertical plane is kept as the world's Y (the same matrix; the
  game's own binary-angle up is a hair off that plane for every view), so only real rolls show.
- **`MREG(64)`'s fill** is drawn where `Play_Draw` draws it: after the actors and the lens flares,
  just before `envCtx.fillScreen`'s, both at the end of the OPA and XLU lists, so it's composed
  under the environment's fill (`PlayState::mreg_fill`, `draw_fills`). In this ROM it never shows
  (below).
- **The hot floor:** a dump of the room's collision: the lava is the 720 square round (-890, -3304)
  at y -1524, eight polygons of `FLOOR_TYPE_3` (`func_80838144`'s second hot floor, which Player's
  port handles with type 2), with a 16-polygon `FLOOR_TYPE_6` rim at -1504 round it (no fall
  damage) and the upper ledge at -744.

### What was built

- **`Boss_Dodongo`** whole (ADR 0075): his intro (states 0 to 4: the fall's trigger below -1223.76,
  Link's cutscene actions 1, 9, 12 and 7, the sub camera's shake with his steps and its tilt
  through `Play_SetCameraAtEyeUp`, the roar, the title card the first time, `NA_BGM_FIRE_BOSS`,
  `EVENTCHKINF_BEGAN_KING_DODONGO_BATTLE`; a second try starts beside Link), the walk round the
  corners, the roll, the inhale (any `ACTORCAT_EXPLOSIVE` within 40 of his mouth eaten), the fire
  breath, the bomb's blast inside him (2 off his 12, never the last), lying down, a sword's damage
  on his head, a seed or the boomerang stopping the inhale, his death (the camera, the lava's
  cover, the last roll round the corners, the sink, the decay into his object's RAM, the lava
  cooling to rock, the heart, the camera back, the warp, the room's clear flag), his 80 particles,
  the light and the lava's effects, the lava floor's room texture, the cleared room at init. His
  draw: one skeleton bake, each limb's wobble, squash and his tail base's scale on its own bone
  only, his own fog (white while hit, red while down), his tilt and roll; the particles' bake.
- **`En_Bdfire`** whole (the flame at his mouth with its light; the fire balls rolling, turning at
  the walls, setting Link burning), **`Effect_Ss_G_Magma`** and **`Effect_Ss_G_Magma2`** whole.
- **`Door_Warp1`'s branch** for his room (and Barinade's): the Ruby, `ENTR_DEATH_MOUNTAIN_TRAIL_0`
  with `CS_INDEX_1` the first time, `_5` after.
- **`Demo_Du`** (Darunia in the cutscenes: the Ruby's part whole, its other cutscenes placeholders
  chosen at init, the user's call) and **`Demo_Go`** (the Ruby's three Gorons) whole, by a helper
  in a worktree (removed); `ActorShadow_DrawCircle`'s checks moved to
  `oot_game::draw::actor_shadow_circle` for them.
- **Systems:** `RoomRam`; the camera's up; `mreg_fill`; King Dodongo's title card in `BOSS_NAMES`;
  King Dodongo's sounds and `NA_BGM_FIRE_BOSS`.
- **The runs:** `Route::KingDodongo`, `Route::Ruby`, `Task::WarpCutscene` (a warp's arrival
  cutscene, with its own limit: the Ruby's texts hold it); the steps `FloorBombed`,
  `KingDodongoBattle`, `RubyOver`, `Trail5`. **The presets** `cavern-boss-fight` (the intro run's
  end), `cavern-boss-beaten` (a debug start: the room cleared), `goron-ruby` (the Ruby run's end:
  the heart container and its flag, `EVENTCHKINF_25`, the Ruby), the first and last checked
  against their runs.
- **The characters** (`export-glb`): King Dodongo (checked in Blender). `Demo_Du`'s Darunia and
  `Demo_Go`'s Gorons are `En_Du`'s and `En_Go2`'s bakes (the helper compared them): their rows.

### Results

- Tests from the C: `tests/boss_dodongo.rs` (18: his wait, the intro and its camera's up, a second
  try, a bomb eaten and its blast, the last bomb, the sword and the seed, his death to the warp,
  the lava floor's wobble and its room texture, the wobble's shifts, the cleared room at init, the
  fill, the decay table, the fire balls, the flame, the lava's effects, the warp's second time, the
  bakes, room 1's floor texture's source), `demo_du.rs` (7), `demo_go.rs` (7), `king_dodongo.rs`
  (2: the runs).
- The runs: the intro 731 frames (the bomb out at 78, put down at 122, the floor broken at 177,
  the fight's start at 731); the Ruby 3,057 frames (the heart at 289, into the warp at 395, out at
  548, on the trail at 631, the cutscene's terminator at 3,005 after its eight texts, the trail's
  spawn 5 at 3,057). No hit taken in either.
- Goldens: 224 of milestone 3's 236 the same; 12 differ, all from the camera's up now in the view
  (proven below). 5 new cases (`kingdodongo`, `kingdodongo_title`, `ruby`, `ruby_warp`,
  `ruby_jewel`). 243 hashes, 185 cases (`out/golden_base27`), the same over two runs.
- 1,315 tests (one ignored) pass; `cargo test -p layering` passes; no warnings. The pack is still
  format 29 (`out/data26`), re-imported for the new bakes.

### Decisions

- The camera's up modelled (above) rather than logged.
- The runs end where the hand test starts (the fight) and resume after his defeat from a debug
  start; the fight and his death in play are the hand test's, his death's sequence checked from
  the C in `tests/boss_dodongo.rs` (his health set to 0).
- `Demo_Du`'s other cutscenes placeholders chosen at init (the user's call, BACKLOG #83).
- The `actor_setup_xlu_dl` copies stay file-local (milestone 3's decision).

### Corrections to the survey

- **`MREG(64)`'s red flush never shows in this ROM:** `unk_244` is only stepped towards 0
  (`BossDodongo_BlowFire` and his update), never set above it. Ported as written.
- His overlay's lines: 1,738 and 240 of data (the survey's figure); `En_Bdfire`'s and his profile
  ids are `@bug (game)`: `ACTOR_EN_DODONGO` and 0 (`ACTOR_PLAYER`), so his actor's `id` is the
  Dodongo's and every fire's is Player's.
- **The floor texture's offset in this ROM is 0x21D8** (`ddan_boss_v2.xml`, the one
  `baseroms/gc-eu-mq-dbg/config.yml` names), not `ddan_boss.xml`'s 0x21C8.
- `Effect_Ss_G_Magma2` faces the camera: its list loads the billboard from segment 1.
- The decay's table has 254 entries; step 256 reads on into the overlay's padding and collider
  data (3: a last write, `@bug (game)`; one lands on the title card's first texels, not modelled).
- The cutscene itself (`gDMTGoronRubyCs`) needed nothing new: every command, cue and text ran
  (the helper's run); its script runs to frame 1,515 over 2,457 game frames (its eight texts).

### Known gaps

- The fight is the hand test's (no run fights him).
- No actor shadows but the cutscene actors' stand-in (#84); no rumble (#60).
- `Demo_Du`'s other cutscenes (#83).
- A lava floor written while another room is current (the C writes that room's buffer) logs.
- Milestone 5's: the rest of the cavern (`En_Dodongo`, `En_Vm`, `En_G_Switch`, `Bg_Ddan_Kd`,
  `D_Fire`, `Dead_Dd`, `Dead_Ds`, one-point 3050, rooms 2, 5, 6, 13, 14, 15); milestone 6's: the
  summit and magic.

### Fixes found while building

- `Door_Warp1`'s logged branch started the transition with the last entrance; it goes where the
  C sends it now.
- The boulder run's check against `cavern-open` (milestone 2's `tests/goron_city.rs`) had failed
  since milestone 3 put the Hylian Shield's chest flag into that preset: the run doesn't open the
  grave, and the check now says so.
- Python's text mode writes CRLF on Windows: files edited that way were put back to their own
  line endings (the lesson: edit in binary).

### Golden proof

- **The camera's up:** the views now roll where the game rolls them: the one-point cutscenes'
  camera (`CAM_SET_CS_C`, its keyframes' roll: -181, about -1°, in the castle's, the meadow's and
  the farewell's runs) and the scripts' camera roll (-10,752 in the new file's opening). The
  rolled view-projection moves `projectedPos` and the culling volume, and the runs follow from
  there. Shots: `castle_caught_shut`, `creation_rain`, `farewell_owl`, `graveyard`,
  `graveyard_keese`, `graveyard_lesson`, `meadow_wolfos`. Traces: `castle` and `castle_caught`
  (only `current_yaw` and `shape_yaw`, frames 181 to 558: the route's steering after a rolled
  one-point), `farewell` (only `actors`, 5 frames from 3726), `graveyard` (from frame 2806: its
  Keese die three frames earlier and the run follows; `tests/graveyard.rs`'s rupees are 31, from
  33), `new_file_deku_tree` (only `actors`, one frame, 2567).
- **A**, this code with the view's up forced to the world's Y (`target/game26proof`): all 236 of
  milestone 3's hashes back (`out/golden_proof_g9m4_A.txt`).
- Before the 0.1° tolerance (`camera::view_up`), 34 differed: the binary-angle up of a view that
  doesn't roll sits a few thousandths of a degree off its vertical plane (`out/golden_check_m4_1.txt`).

### How to check

```
scripts\run\test-kingdodongo.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-kingdodongo.bat
```

### By hand

1. `game-kingdodongo` (King Dodongo's room, `cavern-boss`: 18 bombs on C-Right): west along the
   ledge to the cracked floor, a bomb (C-Right, A to put it down) on it, back off: the floor
   breaks. Drop through the hole: his intro (the camera follows Link down, then King Dodongo
   walking in from the west, the screen shaking with his steps and tilting a little; his roar,
   "Infernal Dinosaur King Dodongo", the boss music).
2. The fight (or `game-kingdodongo fight`: beside him, his battle begun, a short intro): he walks
   the corners and rolls at Link; with Link ahead of him he stops and inhales: throw a bomb (or
   lift a corner's Bomb Flower and throw it) into his mouth. He swallows it, it blows inside him,
   he lies down red: slash his head. Getting hit by his roll or his fire hurts; the fire sets Link
   alight. The lava (red, wobbling) burns. A seed into him while he inhales stops it.
3. His death (the same run on): the camera on him, his throes, the lava's cover rising, a last
   roll, he rolls into the lava and sinks, crumbling away; the lava cools to rock; the heart
   container; the blue warp. Take the heart (or `game-kingdodongo beaten`), step into the warp.
4. The Goron's Ruby (or `game-kingdodongo ruby`): on Death Mountain Trail, Darunia drops into the
   cavern's mouth, beats his chest, slaps Link, gives the Ruby; three Gorons drop in and walk off;
   Link ends at the trail's spawn 5 (or `game-kingdodongo trail`).
5. Anything that looks off: his look and wobble, the fire, the lava's bubbles and splashes, the
   intro's tilt, Darunia's face and the Gorons. Known: no shadows, no rumble.
