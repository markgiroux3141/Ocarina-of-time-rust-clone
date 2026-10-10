# GAME-06: the road out of the forest

**Goal:** Phase 7 of the [roadmap](ROADMAP.md): from the Deku Tree's death to the world outside
the forest. The cutscene chain after the Kokiri Emerald's first part, Saria's goodbye on the
bridge, Hyrule Field with its clock, its owl and its enemies, and the way into Kakariko and
Castle Town.

**Status:** decided (2026-10-09). The user chose:
- the split below, with Kakariko, Castle Town and Hyrule Castle moved to Phase 8;
- the whole chain played, `Demo_Effect` and `Demo_Kankyo` ported whole;
- for milestone 2, the parts of `Demo_Sa` and `Obj_Bean` this phase reaches (the rest logged), and
  `En_Owl` whole;
- the lens flare's occlusion through an engine depth probe: the depth at one pixel, read back
  each frame and used the next, as the C reads its depth buffer.

| # | Milestone | Status |
|---|---|---|
| 1 | The Kokiri Emerald's cutscene chain after part 1 (BACKLOG #23), in two: 1a the emerald and Farore's light (`Demo_Effect`, Kokiri Forest's layers 4 and 6); 1b the creation (`Demo_Kankyo`, `Bg_Spot09_Obj`, `Bg_Spot16_Doughnut`, two scene draw configs, two skyboxes) and the whole chain to `ENTR_KOKIRI_FOREST_11` | done |
| 2 | Leaving the forest: Kokiri Forest after the emerald, the Lost Woods bridge, Saria's goodbye (`Demo_Sa`) and the Fairy Ocarina, Hyrule Field's intro and the owl (`En_Owl`) | done |
| 3 | The clock, day and night: time passing (`Environment_Update`), the sky by the time of day, the sun and the moon, the lights, the night's music and ambience | done |
| 4 | Hyrule Field: its props (trees, bush and rock circles, signs, grottos' holes) and its enemies (`En_Peehat` by day; `En_Encount1` and `En_Skb`, the Stalchildren, by night) | done |

The working rules are the same as for the earlier phases:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack, and only the importer reads the decomp;
- ports go function by function with the decomp's names, every constant is cited, and faithful
  bugs are marked `@bug (game)`;
- test expectations come from the C;
- a change in a run's trace is a golden change to explain.

The decomp is zeldaret/oot main at `52a510f` ([ADR 0031](adr/0031-decomp-main.md)). Decisions
are in [docs/adr/](adr/README.md) (0053 on).

## The survey

Line counts are the decomp's (`52a510f`), whole `.c` files unless said otherwise. "Ported" means
`oot_actors::overlays()` registers the actor today (`84301df`, the end of GAME-05). Each scene
layer's actors are the pack's (`ootx scene-info --scene <file> --layer N`).

### 1. The cutscene chain after part 1

The blue warp out of Gohma's room plays Kokiri Forest's layer 5, part 1 (GAME-05 milestone 6b).
Its terminator starts a chain of eight more cutscenes in five scenes. Each part is its scene's
cutscene layer; each terminator names the next (`Cutscene_Command_Destination`, `z_demo.c`).

| Part | Scene, layer | Script | Its terminator |
|---|---|---|---|
| 1 | Kokiri Forest (`spot04`), 5 | `gKokiriForestKokiriEmeraldPart1Cs`: Link arrives by blue warp, the Deku Tree's texts | `CS_DEST_CUTSCENE_MAP_GANON_HORSE`, frame 565: cutscene map, `CS_INDEX_1`, fade to black |
| 2 | Cutscene map (`hiral_demo`), 5 | `gCutsceneMapKokiriEmeraldPart2Cs`: Ganondorf on his horse, lightning | `CS_DEST_CUTSCENE_MAP_THREE_GODDESSES`, 360: cutscene map, `CS_INDEX_0`, white |
| 3 | Cutscene map, 4 | `gCutsceneMapKokiriEmeraldPart3Cs`: the three goddesses' lights descend, texts 0x107F to 0x1083 | `CS_DEST_GERUDO_VALLEY_DIN_PART_1`, 813: Gerudo Valley, `CS_INDEX_1`, white |
| 4 | Gerudo Valley (`spot09`), 5 | `gGerudoValleyKokiriEmeraldPart4Cs`: Din's light, the rocks thrown up, text 0x1084 | `CS_DEST_GERUDO_VALLEY_DIN_PART_2`, 114: Gerudo Valley, `CS_INDEX_0`, brown |
| 5 | Gerudo Valley, 4 | `gGerudoValleyKokiriEmeraldPart5Cs`: the camera down the valley, lightning, text 0x1085 | `CS_DEST_DEATH_MOUNTAIN_TRAIL_NAYRU`, 110: Death Mountain Trail, `CS_INDEX_0`, white |
| 6 | Death Mountain Trail (`spot16`), 4 | `gDMTKokiriEmeraldPart6Cs`: Nayru's light, texts 0x1086, 0x1087 | `CS_DEST_KOKIRI_FOREST_FARORE`, 228: Kokiri Forest, `CS_INDEX_0`, white |
| 7 | Kokiri Forest, 4 | `gKokiriForestKokiriEmeraldPart7Cs`: Farore's light, the grass faded in (`CS_MISC_FADE_KOKIRI_GRASS_ENV_ALPHA`), texts 0x1088, 0x1089 | `CS_DEST_CUTSCENE_MAP_TRIFORCE_CREATION`, 190: cutscene map, `CS_INDEX_2`, white |
| 8 | Cutscene map, 6 | `gCutsceneMapKokiriEmeraldPart8Cs`: the goddesses leave, the Triforce, texts 0x108A to 0x108D, `CS_MISC_TIME_ADVANCE_TO_NIGHT` | `CS_DEST_KOKIRI_FOREST_RECEIVE_KOKIRI_EMERALD`, 700: Kokiri Forest, `CS_INDEX_2`, instant |
| 9 | Kokiri Forest, 6 | `gKokiriForestKokiriEmeraldPart9Cs`: the emerald shown, held up, floating over Link; the Deku Tree's death (`CS_MISC_DEKU_TREE_DEATH`), Navi's cues, 12 texts | `CS_DEST_KOKIRI_FOREST_FROM_KOKIRI_EMERALD`, 1170: `ENTR_KOKIRI_FOREST_11`, fade to black |

**What the port has:** every command these nine scripts use. That includes the misc commands
(lightning, the grass's fade, the Deku Tree's death, the time advance), the lighting commands,
`SCENE_TRANS_FX`, the brown and white fills, `FADE_BGM`, the texts and Player's cues. Every scene's
rooms, collision and objects are in the pack. The user played the chain by hand after milestone
6b: it runs to its end, with placeholders for the actors below.

**The chain's actors:**

| Actor | Lines | Ported | Where | What |
|---|---|---|---|---|
| `Demo_Effect` | 2,092 | no | cutscene map 4 (`0xF404`, `0xF505`, `0xF606`: Din's, Nayru's and Farore's lights), 6 (those and `0xFF08`: the Triforce); Death Mountain Trail 4 (`0xFF05`, Nayru); Kokiri Forest 4 (`0xFF06`, Farore), 6 (`0x0013`: the Kokiri Emerald; `0x2112`: a light) | 26 effect types on 21 update functions. The chain reaches 12: the three god lights and what they spawn (Din's fire balls, Nayru's expanding light rings, Farore's light shower), the Triforce spot with its crystal light, light ring and blue orb, the emerald (`DemoEffect_UpdateJewelChild`), the light. Not reached here: the six medals, the Master Sword's and the Song of Time blocks' time warps, the light arrow, the dust, the Goron and Zora jewels. The Song of Time blocks' time warp **is** reached in this ROM's Deku Tree (`Obj_Timeblock` spawns it; a placeholder since GAME-05 milestone 4c, ADR 0044) |
| `Demo_Kankyo` | 1,230 | no | cutscene map 4 and 6 (`0x0000`); Gerudo Valley 5 (`0x0002` to `0x0006`) | 18 types. The chain reaches `DEMOKANKYO_BLUE_RAIN` (the creation's falling light) and `ROCK_1` to `ROCK_5` (Din's rocks, on cues). Not reached: the clouds, the Door of Time, the light plane, the warp songs' sparkles in and out |
| `Bg_Spot09_Obj` | 194 | no | Gerudo Valley 4, 5 (6 placed) | The valley's bridge pieces and tent, by age and flags |
| `Bg_Spot16_Doughnut` | 170 | no | Death Mountain Trail 4 | The summit's cloud ring |
| `En_Viewer` | 914 | yes (GAME-04b) | cutscene map 5 (`0x05FF`, `0x06FF`) | Ganondorf and his horse, as in the opening's nightmare |
| `En_River_Sound`, `Bg_Treemouth`, `Object_Kankyo`, `En_Holl` | | yes | Gerudo Valley; Kokiri Forest | |

**Beyond the actors:**
- **Scene draw configs:** `Scene_DrawConfigGerudoValley` (about 33 lines) and
  `Scene_DrawConfigDeathMountainTrail` (33). The cutscene map's is `SDC_DEFAULT`, ported.
- **Skyboxes:** the cutscene map's layers 4 and 6 draw `SKYBOX_CUTSCENE_MAP` (5); its layer 5 and
  Death Mountain Trail draw `SKYBOX_NORMAL_SKY` (1), the sky the clock drives (section 3). The
  port draws only the prerendered rooms' skyboxes (ADR 0014); outdoor scenes show the fog's
  colour behind them. Gerudo Valley and Kokiri Forest set none (29).
- **BACKLOG #24:** the god lights and the light rings scroll two tiles in their combiners; if they
  blend by `LOD_FRACTION` as the warp's portal does, #24 is in the way here too.

### 2. Leaving the forest

**After the chain,** Link stands at `ENTR_KOKIRI_FOREST_11` (spawn 11, (1790, 0, 135)) with
`EVENTCHKINF_07` and `_09` and the Kokiri Emerald. Kokiri Forest's child layer 0 changes with the
emerald:
- the Kokiri at the Lost Woods' entrance (`En_Ko` child 3) moves aside to its path's last point;
  the port logs this because GAME-02's `En_Ko` predates the paths in the pack (GAME-03 milestone
  2): a small fix;
- Mido and the others change texts and places (`En_Ko`, `En_Md`: ported, checked by tests).

**The exit:** Kokiri Forest's exit 3 is `ENTR_LOST_WOODS_9`, the bridge (Lost Woods room 5).
`Play_Init` runs `Cutscene_HandleConditionalTriggers` (ported, GAME-03): on that entrance without
`EVENTCHKINF_C1`, it sets the flag, gives the Fairy Ocarina (`Item_Give`), and enters
`ENTR_LOST_WOODS_0` with `CS_INDEX_0`: the Lost Woods' layer 4, `gLostWoodsFairyOcarinaCs`.
Saria waits on the bridge; Link's five cue lists, Saria's (cue 0x40, channel 1) and Navi's
(channel 8); texts 0x1011 to 0x1014 and the ocarina's 0x004A; its terminator
(`CS_DEST_HYRULE_FIELD_FROM_FAIRY_OCARINA`, frame 1130) enters `ENTR_HYRULE_FIELD_3`, fading to
black. Later visits enter the bridge normally and walk on to its exit 9, the same entrance.

| Actor | Lines | Ported | What |
|---|---|---|---|
| `Demo_Sa` | 1,066 | no | Saria in cutscenes. Its params pick one of five uses: the bridge (`0x0005`: `DemoSa_Action_BridgeInvisible` to `_BridgeGiveOcarina`, about 250 lines with the shared helpers), the Forest Medallion, the sages' magic, the credits and an unused one. Only the bridge is reached |
| `Obj_Bean` | 962 | no | The soft soil patch in the layer (`0x1F04`). Most of it is planting a bean and the adult's leaf ride; a child with no beans sees the patch only |
| `En_Shopnuts`, `Object_Kankyo`, `En_Holl` | | yes | |

Also needed: `Scene_DrawConfigLostWoods` (29 lines).

**Hyrule Field's arrival:** `ENTR_HYRULE_FIELD_3` (spawn 3, (5105, -160, 8467)) is in
`sEntranceCutsceneTable` (`EVENTCHKINF_A0`, either age): `gHyruleFieldIntroCs`, a camera sweep
over the field with two misc commands and the place name. The entrance triggers are ported
(GAME-03 milestone 4).

**The owl** (`En_Owl`, 1,437 lines, 99 functions) waits at (4284, 182, 7877), a few steps from the
arrival: `0x004C`, `OWL_OUTSIDE_KOKIRI` with switch flag 0x0C. He talks (texts 0x2064 to 0x2072,
choices to hear it again), then flies off on one-point cutscene 8700. The field places three more
(`OWL_KAKARIKO` by the village's stairs, `OWL_HYLIA_GERUDO`, `OWL_LAKE_HYLIA`), Hyrule Castle one
(`OWL_HYRULE_CASTLE`); the Lost Woods' and the mountain's (which carry Link) come later. Each type
is a wait function and its texts; the flying, perching and talking are shared.

### 3. Hyrule Field

**Its size:** one room (`spot00_room_0`) of about 16,000 by 15,000 units, 1,579 collision polygons,
18 spawns and 16 exits (Castle Town's drawbridge and its side entrance, Kakariko, Zora's River,
Lost Woods, Lake Hylia, Gerudo Valley, Lon Lon Ranch and its three doors, the fairy fountain, the
grottos). Its draw config (`SDC_HYRULE_FIELD`) and the drawbridge (`Bg_Spot00_Hanebasi`, with its
day and night) are ported for the opening's nightmare (GAME-04b milestone 6). Its sky is
`SKYBOX_NORMAL_SKY`; its music is `NA_BGM_FIELD_LOGIC` (sequence 2), whose enemy mode the port
already has (`Audio_SetSequenceMode`, GAME-04).

**Its actors.** A child always gets layer 0 here, by day and by night: `Play_Init` overrides the
night layer for Hyrule Field, and picks layer 1 only once Link holds the three Spiritual Stones
(Zelda's escape: `En_Mm` and `Item_Ocarina` instead of the owls). The port's `Play_Init` doesn't
have that override yet. Night shows only through the actors' own `IS_DAY` checks.

| Actor | Placed | Lines | Ported | What |
|---|---|---|---|---|
| `En_Wood02` | 27 | 475 | no | Trees and bushes; cut bushes drop items |
| `Door_Ana` | 8 | 195 | no | Grottos' holes: the open ones lead to `ENTR_GROTTOS_*` (scenes of their own), the hidden ones open to bombs or the Song of Storms |
| `En_Peehat` | 7 | 1,108 | no | Peahats: 5 grounded (`0xFFFF`), 2 flying (`0x0000`) that drop larvae (`PEAHAT_TYPE_LARVA`, the same file). By day they rise and attack; by night they stay buried (`IS_DAY` in six places) |
| `En_A_Obj` | 7 | 380 (`z_en_a_keep.c`, in `code`) | no | Signposts |
| `Obj_Mure2` | 7 | 229 | no | Circles of bushes and rocks: spawns `En_Kusa` and `En_Ishi` (ported) |
| `En_Owl` | 4 | 1,437 | no | Above |
| `En_Encount1` | 1 | 349 | no | `0x10A4`: `SPAWNER_STALCHILDREN` at (116, 192, 6206), up to 2 at once, endless in Hyrule Field. By night it spawns `En_Skb` near Link; by day (or with the Bunny Hood) it stops |
| `En_Skb` | spawned | 590 | no | The Stalchildren: they rise, walk at Link, attack, and sink back by day or 800 from home |
| `Shot_Sun` | 1 | 225 | no | `0xFF41`: the Sun's Song's fairy spot (an ocarina song: logged) |
| `En_Mm` | layer 1 | 613 | no | The running man (after the three stones: not this phase) |
| `Item_Ocarina` | layer 1 | 213 | no | The Ocarina of Time in the castle's moat after Zelda's escape (not this phase) |
| `En_River_Sound`, `En_Wonder_Item`, `Obj_Bombiwa`, `Bg_Spot00_Hanebasi`, `En_Ishi` | 3, 3, 3, 1, 1 | | yes | |

No Guays (`En_Crow`, 527 lines) are placed in a child's Hyrule Field in this ROM; Leevers,
Tektites and Wolfos are other scenes' `En_Encount1` kinds.

**The enemies' total:** `En_Peehat`, `En_Encount1` and `En_Skb`, about 2,050 lines. The props:
about 1,280 (`En_Wood02`, `Obj_Mure2`, `En_A_Obj`, `Door_Ana`'s hole without the grottos).

**The clock and day and night.** Hyrule Field's room header sets no hour and a time speed of
10 (`SCENE_CMD_TIME_SETTINGS`), on layers 0 to 2; its sky is `SKYBOX_NORMAL_SKY` with
`skyboxConfig` 0 and `LIGHT_MODE_TIME`. At 20 frames a second, a day lasts about 327 seconds:
157 of day and 85 of night (time runs twice as fast at night). What the C does, and what the port
has:

| Part | C | Lines | Port today |
|---|---|---|---|
| Time passing | `Environment_Update`'s clock (`z_kankyo.c` 945 to 1012): `dayTime += gTimeSpeed` (doubled at night), held while paused, in a message, at the game over, while the skybox changes or during a transition; `skyboxTime`; `nightFlag` every frame (`IS_DAY` reads only it); `nextDayTime`. `Scene_CommandTimeSettings` (45 lines) sets `gTimeSpeed` | about 90 | The save's `dayTime` and `nightFlag`, `IS_DAY`, `nightFlag` and the layer at `Play_Init`. `env::scene_times` reads the speed, and both callers throw it away (`play_scene.rs` 189, 470). `EnvCtx::update_lights` assumes `gTimeSpeed` 0 for `skyboxTime`. Nothing advances the clock |
| The lights by the time | `LIGHT_MODE_TIME`'s blend of `sTimeBasedLightConfigs` by `skyboxTime`, the sun's and moon's light directions | | **Ported** (`env.rs` 320 to 438, every frame), waiting for the clock. `EnvCtx::init` leaves `skyboxConfig` 0 instead of the scene's (`Scene_CommandSkyboxSettings`). Hyrule Field's draw config reads a `day_time` frozen at the scene's load |
| The sky | `Environment_UpdateSkybox` (158; the normal sky's branch about 130, with the texture loads' state machine), `Environment_UpdateStorm` (38), `Skybox_Setup`'s `SKYBOX_NORMAL_SKY` case (66: two of `vr_fine0` to `3`, `vr_cloud0` to `3`, `vr_holy0`, and two palette halves into one), `Skybox_Calculate128` and `_CalculateFace128` (172), `Skybox_Init` (42), `Skybox_Draw`'s 128 path (about 30: two CI8 tiles blended by the primitive alpha, two-cycle), the sky's slow turn | about 420 | Only the prerendered rooms' skyboxes (ADR 0014). The pack has no `vr_fine`, `vr_cloud` or `vr_holy` texels. Outdoors, the background is the fog's colour (`apps/oot/src/rooms.rs` 56) |
| The sun and the moon | `Environment_DrawSunAndMoon` (87), `Environment_DrawSkyboxFilters` (36). No stars are drawn: they're in the night's sky texture | about 125 | No |
| The sun's lens flare | `Environment_DrawSunLensFlare` and `_DrawLensFlare` (215). Its occlusion test reads the depth buffer at the sun's pixel (`Environment_GraphCallback`, `ZBufValToFixedPoint`, about 18) | about 235 | No. The depth read needs the engine: a one-pixel readback each frame, or a stand-in |
| The music and ambience | `Environment_PlaySceneSequence` (57), `Environment_PlayTimeBasedSequence` (94): the dusk fade at 17:10, the night's critters, the dog's howl, the rooster at dawn, the field's morning part | | **Ported** (`audio/scene.rs`), waiting for the clock. Left: `totalDays`, `bgsDayCount` and the dog's and egg's counters at dawn |

Without the flare that is about 640 lines, mostly the sky. The engine has what the sky's blend
needs (`TEXEL1` in the combiner, per-draw segments and textures); two CI8 tiles on one shared
palette is to be checked. For the Stalchildren alone, the clock and `nightFlag` (about 90 lines)
would do.

### 4. Kakariko and Castle Town

Rough sizes, to place them in the plan: the child's layers (day, and night where the scene has
one) of each area's scenes, with their interiors. "Unported" counts each actor kind once, whole
files, `En_Ossan`'s other shopkeepers not included.

| Area | Scenes | Actor kinds, placements | Unported kinds | Unported lines | The biggest |
|---|---|---|---|---|---|
| Kakariko | the village, the graveyard, Impa's house, the potion shop, the guest house, the back alley house, the House of Skulltula, the bazaar, the windmill, a grave | 62, 395 | 42 | about 21,000 | `En_Go2` 2,113, `En_Hy` 1,524 (the townsfolk), `En_Ta` 1,344 (Talon), `En_Niw` 1,247 (the cuccos), `En_Rd` 1,037, `Obj_Bean`, `En_Ssh`, `En_Heishi2`, `En_Tk` (Dampé) |
| Castle Town | the drawbridge's entrance (day, night), the market (day, night), the back alley (day, night), the Temple of Time's outside (day, night) and inside, the shops and games (potions, Bombchus, the chest game, the shooting gallery, Bombchu Bowling, the mask shop), the guard house | 43, 234 | 32 | about 17,000 | `En_Xc` 2,494 (Sheik), `Demo_Effect`, `En_Hy`, `Demo_Kankyo`, `Demo_6K` 820, `En_Gs`, `En_Bom_Bowl_Man`, `En_Dog` 498 |
| Hyrule Castle | the castle grounds, the guards' courtyard (day, night), Zelda's courtyard | 27, 82 | 18 | about 14,000 | `En_Zl4` 1,527 (Zelda), `Demo_Im` 1,477 (Impa), `En_Owl`, `En_Ta`, `En_Heishi2`, `En_Heishi1` 526 (the guards' patrols) |

Shared between them (counted in each row): `Demo_Effect` and `Demo_Kankyo` (milestone 1),
`En_Owl` (milestone 2), `En_Wood02` and `Obj_Mure` (milestone 4), `En_Hy`, `En_Ta`, `En_Ma1`,
`En_Gs`. Their draw configs (`SDC_KAKARIKO_VILLAGE`, `SDC_HYRULE_CASTLE`, `SDC_DEFAULT` for the
town) are small; their houses and shops are prerendered (ADR 0014), where BACKLOG #1 (the edges
past the picture at 16:9) applies.

Together, about 52,000 lines: more than GAME-05 ported in all. They are a phase of their own.

## The proposal

**Kakariko and Castle Town become Phase 8** (GAME-07). Their 52,000 lines would make this phase
twice GAME-05. Phase 7 ends in Hyrule Field, with its exits into them working (their scenes load,
their actors placeholders). That also keeps Hyrule Field's run short enough for one golden.

**The milestones** (each one commit, as GAME-05's):

- **1a, the emerald and Farore's light:** `Demo_Effect` whole (its Song of Time warps replace the
  placeholder `Obj_Timeblock` spawns in the Deku Tree, ADR 0044), Kokiri Forest's two layers (4:
  Farore's light; 6: the emerald shown, held up and floating over Link, the Deku Tree's death).
  These are the two gaps the user found. A debug start with a cutscene index (`--cutscene 0xFFF2`)
  for the game, the sandbox and the routes.
  **Exit:** from Kokiri Forest's layer 6, the emerald over Link through the Deku Tree's death to
  `ENTR_KOKIRI_FOREST_11`: `Route::Emerald`, a golden.
- **1b, the creation:** `Demo_Kankyo` whole, `Bg_Spot09_Obj`, `Bg_Spot16_Doughnut`, Gerudo
  Valley's and Death Mountain Trail's draw configs, `SKYBOX_CUTSCENE_MAP`, and the normal sky
  pulled forward from milestone 3 at the scene's fixed time (parts 2 and 6 draw it); BACKLOG #24 if
  the lights need `LOD_FRACTION`.
  **Exit:** the whole chain as a test, from part 1 (the blue warp's route carried on) to
  `ENTR_KOKIRI_FOREST_11`, each part's scene, cues and texts checked; short goldens of part 3 (the
  goddesses) and part 4 (Din's rocks) from debug starts.
- **2, leaving the forest:** Kokiri Forest after the emerald (`En_Ko` child 3's path), the bridge
  (`Demo_Sa`, `Obj_Bean`, `Scene_DrawConfigLostWoods`), the Fairy Ocarina, Hyrule Field's intro
  and arrival, `Play_Init`'s Hyrule Field layer, `En_Owl`.
  **Exit:** from `ENTR_KOKIRI_FOREST_11` (a preset after the chain), out of the forest, Saria's
  goodbye and the ocarina, the field's intro, the owl's talk and flight: `Route::Farewell`, a
  golden.
- **3, the clock:** time passing, `nightFlag`, the sky by the time of day, the sun and moon, the
  skybox filters, the lens flare (as decided), the dawn's counters; Hyrule Field's draw config fed
  each frame.
  **Exit:** in Hyrule Field from 17:00 (`--time`), dusk to night: the sky, the lights, the music's
  fade and the night's ambience, the drawbridge raised; the clock's rate and holds checked against
  the C by tests; goldens of the field by day, at dusk and by night.
- **4, Hyrule Field:** `En_Wood02`, `Obj_Mure2`, `En_A_Obj`, `Door_Ana` (the holes; the grottos'
  scenes load with placeholders), `En_Peehat` with its larvae, `En_Encount1` and `En_Skb`; the
  field's enemy music.
  **Exit:** a Peahat fought by day, Stalchildren by night (a debug start at 20:00), the way into
  Castle Town's drawbridge and Kakariko's stairs: `Route::Field`, a golden.

**Day and night before Hyrule Field's enemies** (milestone 3 before 4): the Peahats and
Stalchildren read `IS_DAY`, and a clock that runs is needed to see either side of them by hand.
Folding it into milestone 4 would make that milestone about 4,000 lines with an engine question in
it.

**The small items:**
- **#24** (`LOD_FRACTION`) in milestone 1b, if `Demo_Effect`'s or `Demo_Kankyo`'s lists blend by it;
  else with the next list that does.
- **#21 and #22** (the title screen and the file select, F5's reset): about 6,400 lines
  (`z_file_choose.c` and its two companions, `z_title.c`), and the title's demo plays its
  cutscenes in Hyrule Field at set times. After milestone 3 they can be built; they stay out of
  Phase 7 unless chosen, as the menus' own milestone.
- **#1** (the shop's edges at 16:9) with Phase 8's prerendered houses and shops.

**The other choices** (the user's in the status above):
- how much of `Demo_Effect` and `Demo_Kankyo`: whole *(chosen)*, or the chain's types, or only
  Kokiri Forest's parts played;
- `Demo_Sa`, `Obj_Bean`, `En_Owl`: the reached parts with the owl whole *(chosen)*, all three
  whole, or the reached parts only;
- the lens flare's occlusion: an engine depth probe *(chosen)*, a collision stand-in, or logged.

## Milestone 1a: the emerald and Farore's light

**Answer:** done. `Demo_Effect` is ported whole, and Kokiri Forest's two cutscene layers in the
chain play with it. In layer 6, the emerald's last part, a green light grows out of the Deku Tree,
the screen whites out, and the Kokiri Emerald appears over Link's raised hand, turning; the Deku
Tree dies, and Link stands at `ENTR_KOKIRI_FOREST_11`, the meadow's path. In layer 4, Farore's
light flies down over the forest at night and leaves her light shower. The other parts' effects
(the goddesses, the Triforce) are ported too and wait for their scenes (1b). The Song of Time
blocks' time warp, a placeholder since GAME-05 milestone 4c, is real. The exit holds; its run is
the golden `emerald`, with `emerald_light`, `emerald_floats` and `farore`.

The pack is format 28, in `out/data25` (the curve skeletons and animations, `Demo_Effect`'s bakes,
every bake's vertex sources); builds in `target/game26`. Decisions are in
[ADR 0053](adr/0053-demo-effect-curve-skeletons-and-vertices-by-source.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 89 to 91):
- `test-emerald.bat`: the milestone's tests (`Demo_Effect`, the curve skeletons, the
  `--cutscene` start, the run);
- `game-emerald.bat`: Kokiri Forest's layer 6 from its start (`--cutscene 0xFFF2`,
  `deku-tree-dead`); `game-emerald.bat farore` its layer 4 (`0xFFF0`);
- `sandbox-emerald.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** 1a is done; 1b (the creation: the other four scenes, and the whole chain
as a test) is next.

### What was built

**`Demo_Effect`** (`oot_actors::demo_effect`, ADR 0053), whole:
- every type's init (`DemoEffect_Init`, `_InitJewel`, `_InitGetItem`; the Triforce's crystal light
  and its ring spawned under it), `DemoEffect_WaitForObject`;
- the goddesses' lights (`DemoEffect_UpdateGodLgtDin`, `_Nayru`, `_Farore`: along their cues,
  facing them, their sounds by scene and frame) and what they leave: Din's fire balls
  (`_InitCreationFireball`, `_UpdateCreationFireball`: they fall, then burst into a blue orb and
  two rings), Nayru's light rings, Farore's light shower;
- the light rings (`_UpdateLightRingExpanding`, `_Shrinking`, `_Triforce`), the blue orb
  (`_UpdateBlueOrbGrow`, `_Shrink`), the crystal light, the Triforce (`_UpdateTriforceSpot`: its
  fade-in, the column's, the crystal light's);
- the light (`_UpdateLightEffect`), the jewels (`_UpdateJewelChild`, `_Adult`, the Door of Time's
  circling and split, `_PlayJewelSfx` with `sSfxJewelId`), the medals and light arrows
  (`_UpdateGetItem`), the dust;
- the time warps (`_InitTimeWarp`, `_InitTimeWarpTimeblock`, `_UpdateTimeWarpTimeblock`,
  `_ReturnFromChamberOfSages`, `_PullMasterSword`, `_TimewarpShrink`);
- the eleven draws, as bakes (`gCrystalLightDL`, `gCreationFireBallDL`, the goddess's aura and
  body, the light ring, `gEffFlash1DL`, `gEnliveningLightDL`, the Triforce's column and spot (fading
  and solid), `gTimeWarpDL`, the three jewels' gems and settings), their scrolls
  (`Gfx_TwoTexScroll`, `Gfx_TexScroll`) and colours dynamic; the medals through `GetItem_Draw`.

**Elsewhere:**
- **Curve skeletons** (`oot_game::skel_curve`): `z_fcurve_data.c` and
  `z_fcurve_data_skelanime.c` whole; the importer reads curve skeletons and animations
  (`oot_import::curve`) into `curve/` and `curve_anim/` records.
- **Vertex sources in bakes** (`eng_gfx::Batch::sources`, `eng_gbi`'s `track_vertex_sources`): a
  draw rebuilds the colours of the vertices its object's RAM holds.
- **`--cutscene`** for the game and the sandbox (a debug start's `cutsceneIndex`), and
  `Route::cutscene`.
- `Route::Emerald` (`--script emerald`), with `Task::Emerald` and `Task::NewScene`.
- The sound effects `Demo_Effect` plays (`NA_SE_EV_GOD_FLYING`, `NA_SE_IT_DM_FLYING_GOD_*`,
  `NA_SE_EV_SPIRIT_STONE`, `NA_SE_EV_TRIFORCE`, `NA_SE_EV_AURORA`, ...).

### Results

**Tests.** `cargo test --release --workspace` (`target/game26`, `OOT_DATA_DIR=out/data25`):
748 passed, 0 failed, 1 ignored (731 before); 17 are new. New, with their expectations from the C:
- **`oot_actors --test demo_effect`** (12): every type's init (scales, colours, cue channels, the
  light's colour by its params, the god lights' scale by entrance); the Triforce's children and
  their parents; its fade-in counted from `primXluColor[0]` (8.5 a step to 30, the column's to 60,
  the crystal light's 3.1875 a step to 140), its column's vertex alphas in the object's RAM and in
  its draw, its sounds; Din's fire ball (her facing 0x4000, 50 frames at 1.5 and -0.03, the burst on
  the 51st: an orb at 0, rings at 0.1 and 0.2, `NA_SE_IT_DM_RING_EXPLOSION`); Nayru's rings on
  frames 1, 6 and 11 (rot.x + 0x4000, scale 1); Farore's shower 150 below at (0.23, 0.15, 0.23),
  gone on its 85th frame; the light rings' alphas (at 228: 224; gone past 255; the shrinking ring
  from 351 by 2, 8 at 255, gone under 2); the blue orb's growth in f32 and its 15-frame shrink; the
  light's growth to 40 × 0.05 and its first-draw skip; the emerald hidden on cue 1, its sound from
  one jewel only; the Song of Time block's warp (frame 3.55 after its first update, limb 1's y scale
  by the cubic, 22 frames to 59, the alphas 101 and 127 at its 50th shrinking frame, put back and
  gone on its 101st); the bakes' vertex sources (`gTimeWarpVtx`'s 21, `gTriforceVtx[86..96]`); the
  exit's run.
- **`oot_game` `skel_curve::tests`** (4): `Curve_Interpolate`'s three kinds and its ends, the
  Hermite basis, `SkelCurve_Update`'s scales (the `u16` constant 0xFFFF stored as -1), the limb walk.
- **`oot --test start`** (1): `--cutscene 0xFFF2` enters Kokiri Forest's layer 6 with its script
  running; refused without `--entrance`.
- `oot_import --test pack`: the curve skeletons and animations counted (193 of 194 skeletons, 3 of
  4 curve animations: an overlay's left out).

**The exit run** (`Route::Emerald`, `--script emerald`, `--cutscene 0xFFF2`, `deku-tree-dead`):
`emerald_shown` 898, `tree_death` 1741, `emerald_over` 2159, `forest_11` 2212; texts 0x1029,
0x1093, 0x0080 (the emerald's), 0x102A, 0x102B, 0x102C; `NA_SE_EV_SPIRIT_STONE`,
`NA_SE_EV_WHITE_OUT` and `NA_SE_EV_DEKU_DEATH` heard.

**The goldens.** Against 6c's build (`target/game25`, on data24: 121/121 identical, its outputs in
`out/golden_base7`): every hash the same bytes. New: `emerald` (the trace and the end, Link at
`ENTR_KOKIRI_FOREST_11`), `emerald_light` (frame 885: the green light out of the Deku Tree),
`emerald_floats` (1100: the emerald over Link), `farore` (layer 4, frame 30: Farore's light),
each the same bytes over two runs. Logged in [golden/README.md](../golden/README.md): 126 hashes,
93 cases.

### Decisions

- **[ADR 0053](adr/0053-demo-effect-curve-skeletons-and-vertices-by-source.md):** `Demo_Effect`
  whole, its union kept as bytes; curve skeletons whole, as records of their own; bakes record
  their vertices' sources so a draw rebuilds the colours of vertices the game writes; the prim
  colour's LOD fraction baked from the draw's own command; `--cutscene` for a debug start.

### Known gaps

- **The chain's other parts** (the cutscene map, Gerudo Valley, Death Mountain Trail) still play
  with placeholders for `Demo_Kankyo`, `Bg_Spot09_Obj` and `Bg_Spot16_Doughnut`, and with no sky:
  milestone 1b.
- **The sparkles** of the jewels at the Door of Time and of the medals
  (`EffectSsKiraKira_SpawnDispersed`) aren't drawn; their `Rand` calls are made (as for Navi's,
  BACKLOG #9).
- **The jewels' and the Triforce's shine** (`func_8002EBCC`, `func_8002ED80`'s look-at): the
  renderer's texgen reads the view, as for the Gold Skulltula (the roadmap's rendering debt).
- **The Song of Time blocks' warp** is reached only in the tests: the ocarina isn't ported.
- **The chest's light** (`Demo_Tre_Lgt`) still isn't drawn, though the curve skeletons it needs
  are in now (BACKLOG #25).
- BACKLOG #24 (the warp portal's `LOD_FRACTION`) stands: `Demo_Effect`'s lists blend by
  `PRIM_LOD_FRAC` instead, which the bakes now carry.

### Fixes found while building

- **The prim colour's LOD fraction:** the dynamic colour segments bake `gDPSetPrimColor`'s `l` as
  0, which would drop the second tile of every list that blends by `PRIM_LOD_FRAC`. `Demo_Effect`'s
  bakes carry the draw's own command (ADR 0053).
- **The run's end:** its first end screenshot was black (the scene had changed, the fade still
  running); the route now settles in the new scene (`Task::NewScene`).

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-emerald.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-emerald.bat
```

### By hand

`game-emerald.bat` (or menu 90). Space A, WASD the stick.
1. **The texts:** the Deku Tree speaks; Space through each text.
2. **The emerald:** a green light grows on the Deku Tree's face and the screen whites out; then
   the Kokiri Emerald (a green gem in a gold setting) hangs over Link's raised hand, turning
   slowly, with a soft chime.
3. **The death:** the Deku Tree's last texts; he greys as he dies, with a long creak.
4. **After:** a fade to black, then Link stands on the path out of the meadow, "Kokiri Forest"
   shown; you can walk.
5. **Farore:** `game-emerald.bat farore` (menu 90, then `farore`): night over the forest;
   Farore's green light streaks down, a green dome of light spreads and fades, the grass fades in,
   and her texts.
6. **The chain:** `game-blue-warp.bat` plays it from the warp: the forest's parts are the ones
   above; the cutscene map, Gerudo Valley and Death Mountain parts still lack their effects and sky
   (1b).
- **What to report:** the emerald's look (its shine, its setting, its size and height over Link,
  its turn), the green light's size and colour, the white out's timing, Farore's light and dome,
  and the sounds (the emerald's chime, the goddess's rush).

## Milestone 1b: the creation

**Answer:** done. The whole Kokiri Emerald chain plays, from the blue warp to
`ENTR_KOKIRI_FOREST_11`, and BACKLOG #23 is closed. After part 1 the cutscene map shows Ganondorf
on his horse under its own turning sky; then the goddesses descend through a blue rain. Din's
rocks tumble through the fog over Gerudo Valley, with the room hidden as in the game. The valley
follows, then Nayru's light rings in Death Mountain Trail's blue sky, Farore over the forest, the
Triforce in the cutscene map, and the emerald and the Deku Tree's death. The exit holds: the
chain runs as a test from Queen Gohma's room, and the creation's run is the golden `creation`,
with `creation_rain`, `creation_rocks` and `creation_nayru`.

The pack is still format 28 in `out/data25`, re-imported for the new bakes (`Demo_Kankyo`'s, the
cloud ring's, and 56 sky bakes); builds in `target/game26`. Decisions are in
[ADR 0054](adr/0054-the-128-skies-and-demo-kankyo.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 92 to 94):
- `test-creation.bat`: the milestone's tests (`Demo_Kankyo`, the valley's bridges, the cloud
  ring, the normal sky's loads, the two draw configs, the runs);
- `game-creation.bat`: the cutscene map's layer 5 from its start (`--cutscene 0xFFF1`,
  `deku-tree-dead`), the creation part by part to the emerald; or one part: `goddesses`, `din`,
  `valley`, `nayru`, `triforce`;
- `sandbox-creation.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** milestone 1 is done; 2 (leaving the forest: Saria's goodbye, the Fairy
Ocarina, Hyrule Field's intro and the owl) is next.

### What was built

**`Demo_Kankyo`** (`oot_actors::demo_kankyo`, ADR 0054), whole:
- every type's init (`DemoKankyo_Init`: the rain's speed and scale by scene, killed elsewhere; the
  rocks' scale and tumble; the clouds' angles and speeds; the Door of Time's `Door_Toki` child or,
  opened, `roomCtx.drawParams[1]`; the warps' category, room and timer; Saria's melody);
- the actions: `DemoKankyo_SetupType` (the object's slot, then the type's action; the warp-out's
  white fade and its `gChildWarpInCS` or `gAdultWarpInCS`, by the Temple of Time;
  the warp-in's scripts), `_UpdateRock` (`_SetPosFromCue` on channel `params - ROCK_1`),
  `_UpdateClouds`, `_UpdateDoorOfTime` (102 frames, `EVENTCHKINF_OPENED_DOOR_OF_TIME`),
  `_KillDoorOfTimeCollision`, `_UpdateWarpIn`, `_DoNothing`;
- the draws: the rain (`func_80989B54`; 30 streaks of 5 drops kept 350 ahead of the eye, falling
  in the cutscene map and rising elsewhere; the Temple of Time's conditions), the rocks, the
  clouds, the Door of Time's halves, the light plane, the warp sparkles and the sparkles (their
  splines through `func_800BB2B4`, `DemoKankyo_Vec3fAddPosRot`, the sizes, the colours by song or
  respawn data, `Environment_WarpSongLeave` at the warp-out's end, the actor's end);
- the draws as bakes: the rain (`object_efc_star_field_DL_000080`, `SETUPDL_20`, its colours
  dynamic), the rocks (`_DL_000DE0`), the clouds (`gEffDustDL` with `gDust5Tex`, `SETUPDL_61`), the
  Door of Time (`object_toki_objects_DL_007440`, `_007578`), the light plane (`_008390`, its
  scroll dynamic); the sparkles reuse `Demo_Effect`'s flash.

**The outdoor skies** (`oot_game::skybox`, `oot_game::env`, ADR 0054):
- `Skybox_CalculateFace128` and `Skybox_Draw`'s 128 sky written out and baked
  (`skybox::bake_128`): the cutscene map's (six faces), the overcast sunset's, and the normal sky's
  pairs (`normal_sky_pairs`), 56 bakes; `BakeSegment::Files` for the two palettes in one buffer;
- `SkyboxContext`: `Skybox_Init` and `Skybox_Setup` (the pair by `skyboxTime`, a kept storm's
  config), the turn (`skyboxCtx.rot`), the loads received;
- `Environment_UpdateSkybox` (the time-based entry, a config change's blend,
  `Environment_UpdateStorm`, the DMA states a call apart) at `Play_Draw`; the importer reads
  `gTimeBasedSkyboxConfigs` and `gNormalSkyFiles`;
- the app draws the sky first, at the eye, its blend dynamic.

**Elsewhere:**
- **A hidden room:** `roomCtx.curRoom.segment = NULL` (the rain, the rocks, `CS_MISC_HIDE_ROOM`)
  clears `Room::loaded` until the next room load.
- **`Bg_Spot09_Obj`** (`oot_actors::bg_spot09_obj`), whole: `func_808B1C70`'s checks (the
  collision first, then whether this one is here by layer, age and the carpenters, then its
  scale), its draws (the tent's entrance translucent).
- **`Bg_Spot16_Doughnut`** (`oot_actors::bg_spot16_doughnut`), whole: the ring's scale by scene,
  fiery for an adult before `EVENTCHKINF_2F` and faded out on channel 2's cue 2, the expanding
  rings; its two draws as bakes.
- **The scene draw configs** `Scene_DrawConfigGerudoValley` and
  `Scene_DrawConfigDeathMountainTrail` (`oot_game::scene_table`).
- `Route::Creation` (`--script creation`, `Task::Chain`), the chain's steps.
- The sounds `Demo_Kankyo` plays (`NA_SE_EV_STONE_STATUE_OPEN`, `NA_SE_EV_STONEDOOR_STOP`,
  `NA_SE_EV_LINK_WARP_OUT`).

### Results

**Tests.** `cargo test --release --workspace` (`target/game26`, `OOT_DATA_DIR=out/data25`):
763 passed, 0 failed, 1 ignored (748 before); 15 are new. New, with their expectations from the C:
- **`oot_actors --test demo_kankyo`** (14): the cutscene map's rain (the room hidden, `D_8098CF80`
  10 and `sRainScale` 8; no draw until the object's slot is taken; 150 drops, each streak 350
  ahead of the eye, its start in ±250 and [0, 500), its speed 10 to 50, the first drop's matrix
  at 0.008, the next 1500 back and 4000 up; the fall by its speed, the restart 300 under the eye's
  line at 500); the rain killed elsewhere; Din's rocks (the room hidden, the scale and steps in
  range, the cue's halfway point, the tumble added each frame); the clouds' turn by `(s16)`
  speed; the Door of Time (cutscene flag 2, a unit a frame, at 102 the flag, the sound and the
  child gone; opened, `drawParams[1]` 0xFF and no actor); the warp-out's white (0 at timer 20,
  153 at 17, 255 at 15 and 14, 153 at 10, 0 at 4; 17 frames), then the warp-in script and
  `D_8098CF84` 32 on its 34th update; the warp sparkles (two a frame to 30, serenade's colour,
  the first's leave: the return entrance in a white fade, `respawnFlag` -3,
  `EVENTCHKINF_A7` for the Temple of Time); the sparkles (one a frame to 20, the fourth colour,
  the actor's end); `DemoKankyo_Vec3fAddPosRot`'s turn; `Bg_Spot09_Obj` by age, layer and the
  carpenters (scales and collision); `Bg_Spot16_Doughnut` (0.1, 0.04 in Kakariko, its turn, the
  fiery ring's fade in 51 frames, the expanding ring's 51 frames and growth); the normal sky's
  loads (texture 1, its palette, texture 2, its palette, two calls each, the palettes' halves,
  the blend); the exit's runs (below).
- **`oot_game` `skybox::tests`** (1): the 128 sky's faces (`Skybox_CalculateFace128`'s tiles, the
  two textures' tiles at TMEM 0 and 0x80) and the palettes' halves by index parity.
- `oot_import --test pack`: Gerudo Valley's and Death Mountain Trail's draw configs against the C
  interpreter (Death Mountain's night alpha stepped before use, as Hyrule Field's).

**The exit runs.** `Route::Creation` (`--script creation`, `--cutscene 0xFFF1`, `deku-tree-dead`):
`ganondorf` 0, `goddesses` 590, `din` 1425, `valley` 1561, `nayru` 1693, `farore` 1943,
`triforce` 2162, `emerald_part9` 2869, `forest_11` 5076; texts 0x107E to 0x108D, then part 9's six,
each part's `TEXT_LIST` in order; the cutscene map's layers 4 and 6 with the rain alone and the
room hidden, Gerudo Valley's layer 5 with Din's five rocks and the room hidden, its layer 4 with the
bridge's sides alone, and no cloud ring in Death Mountain Trail's layer 4 (its object isn't in the
layer's list). From Queen Gohma's room (`Route::BlueWarp` carried on by `Route::Creation`): into the
warp 139, out 292, part 1 over 944, then the same parts to `forest_11` at 6042, the Deku Tree's
death flag kept.

**The goldens.** Against 1a's 126 hashes: every one the same bytes; no old case shows a 128 sky
(Kokiri Forest's skybox is `SKYBOX_UNSET_1D`). New: `creation` (the trace and the end, Link at
`ENTR_KOKIRI_FOREST_11`), `creation_rain` (the cutscene map's layer 4, frame 210: the blue rain),
`creation_rocks` (Gerudo Valley's layer 5, frame 75: Din's rocks), `creation_nayru` (Death
Mountain Trail's layer 4, frame 107: Nayru's rings in the sky); each the same bytes over two runs,
and each part the same bytes as the whole run's frame. Logged in
[golden/README.md](../golden/README.md): 131 hashes, 97 cases.

### Decisions

- **[ADR 0054](adr/0054-the-128-skies-and-demo-kankyo.md):** the outdoor skies as bakes, one per
  pair of textures, loaded as `Environment_UpdateSkybox` loads them; `Demo_Kankyo` whole, its
  draws' state in `draw_update`; a hidden room as an unloaded one; the chain's other actors and
  draw configs whole.

### Known gaps

- **The warp songs' warp-out** keeps Link drawn through its white-out
  (`player->actor.draw = NULL` logged), and no sub-timer is cut short (BACKLOG #26). The warp
  sparkles are reached only in the tests until the ocarina.
- **The sky's palettes:** a new texture shows with its own palette at once; the C shows it two
  frames with the old one (BACKLOG #27). The skies stand at the scene's time until milestone 3.
- **The Door of Time** has a placeholder `Door_Toki` (its collision) until the Temple of Time
  (Phase 8).
- The other parts' leftovers from 1a stand: the jewels' and medals' sparkles, the shine's look-at.
- BACKLOG #24 stays open: none of these lists read `LOD_FRACTION`.

### Fixes found while building

- **The sparkles' spline:** a hand-copied `sSparklesCameraPoints` had a row too many; the three
  camera point tables are now generated from the C by a script and checked against it.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-creation.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-creation.bat
```

### By hand

`game-creation.bat` (or menu 93). Space A, WASD the stick.
1. **Ganondorf's part:** the cutscene map's grey, cloudy sky, turning slowly behind the
   narration ("This evil...").
2. **The goddesses:** a dark sky, blue streaks of rain falling past the camera, the narration's
   texts; the goddesses' lights fly down, Din's leaving a ring on the ground.
3. **Din:** a sandy fog with tumbling rocks, no ground; then the valley, its river and
   waterfalls scrolling.
4. **Nayru:** looking up between Death Mountain's cliffs into a blue sky with clouds; her blue
   light rings rise.
5. **Farore, the Triforce, the emerald:** Farore's green light over the forest at night (1a's);
   back in the cutscene map's rain, the three goddesses (pink, blue, green) come down around the
   Triforce; then the emerald and the Deku Tree's death (1a's).
6. **One part:** `game-creation.bat din` (or `goddesses`, `valley`, `nayru`, `triforce`) starts
   there and goes on.
- **What to report:** the rain's look (its colour, density, speed and direction against the
  game's), the rocks' size, colour and tumble, whether the fog hides the room in Din's part as in
  the game, the skies (their colours, their turn, the cutscene map's), Gerudo Valley's water, and
  the sounds through the parts.

## Milestone 2: leaving the forest

**Answer:** done. From where the emerald's chain leaves Link (`ENTR_KOKIRI_FOREST_11`), Mido
blocks the path out of the meadow, says it's all Link's fault, and walks off for good. Kokiri
Forest is open: child 3 by the Lost Woods stands aside now that Link has the emerald. The Lost
Woods' exit goes onto the bridge. There Saria fades in, looks sad, clutches the Fairy Ocarina and
holds it out, and Link takes it out and holds it up. Hyrule Field's intro sweeps over the field
under its sky. Below his perch the owl takes Link into his talk, asks whether to hear it again,
and flies off. The exit holds; its run is the golden `farewell`, with `farewell_bridge`,
`farewell_ocarina` and `farewell_owl`.

The pack is still format 28 in `out/data25`, re-imported for Saria's and the owl's bakes; builds
in `target/game26`. Decisions are in [ADR 0055](adr/0055-leaving-the-forest.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 95 to 97):
- `test-farewell.bat`: the milestone's tests (Saria, the soft soil, the owls, the Kokiri, the
  layers, the Lost Woods' draw config, the run);
- `game-farewell.bat`: Kokiri Forest at `ENTR_KOKIRI_FOREST_11` (`deku-tree-dead`);
  `game-farewell.bat bridge` the bridge, `game-farewell.bat field` Hyrule Field;
- `sandbox-farewell.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** milestones 1 and 2 are done; 3 (the clock: day and night in Hyrule
Field) is next.

### What was built

**`Demo_Sa`** (`oot_actors::demo_sa`, ADR 0055), the bridge's parts:
- `DemoSa_InitBridge` (hidden, sad, her fairy as her child), the bridge's five actions
  (`DemoSa_Action_BridgeInvisible` to `_BridgeGiveOcarina`), the cues on channel 1
  (`DemoSa_CsBridge_CheckNextAction`: 4 hidden, 12 sad (faded in from hidden), 13 clutching
  the ocarina, 14 giving it);
- the shared helpers (`DemoSa_Blink`, `_SetEyes`, `_SetMouth`, `_UpdateBgCheckInfo`,
  `_UpdateSkelAnime`, `_GetCue`, `_CheckForCue`, `_CheckForNoCue`, `_SetStartPosRotFromCue`,
  `_AnimationChange`);
- her draws (`DemoSa_DrawOpa`, `_DrawXlu`, `_OverrideLimbDraw`) as bakes: sad opaque, sad
  translucent, eyes shut with the ocarina in hand. Her other four uses are logged.

**`Obj_Bean`** (`oot_actors::obj_bean`): the child's soft soil (`ObjBean_SetupWaitForBean`,
`_WaitForBean`: its patch, its talk for the magic bean, text 0x2F); planting and the adult's lift
logged.

**`En_Owl`** (`oot_actors::en_owl`), whole:
- every type's init (its flag, the story's conditions), wait, talk (`EnOwl_CheckInitTalk`: taken
  at once within range, the owl's fanfare, one-point cutscene 8700), question and end;
- the flight away (`func_80ACA5C8` on: the wings unfolded, the takeoff, the beats and glide, gone
  6000 away), the carrying owls' (`func_80ACC30C` on, their scenes' scripts) and the cutscene
  owls (`EnOwl_WaitDefault`, cue channel 7);
- the head's turn, bob and tilt and the blinking (`EnOwl_Update`), the limbs turned by them in the
  draw (`EnOwl_OverrideLimbDraw`), the focus at the head (`EnOwl_PostLimbUpdate`);
- the draw as bakes, each skeleton with each eye, after `SETUPDL_37`.

**Elsewhere:**
- **One-point cutscene 8700** (`oot_game::onepoint`): the owl's talk's camera.
- **`Camera_SetFinishedFlag` whole** (`PlayState::camera_set_finished_flag`): the active camera
  told too; Player's `CamDone` request uses it.
- **Link's ocarina in a cutscene:** `func_80851D2C` (taken out) and `func_808526EC` (held up,
  its sparkles' `Rand` calls made), with `Player::models_group` for `Player_SetModels`' group.
- `En_Ko` child 3 at his path's last point (`Path_CopyLastPoint`); `Play_Init`'s layer rules
  whole; `Scene_DrawConfigLostWoods` (Cojiro's cry through `DrawConfigState`).
- `Route::Farewell` (`--script farewell`): `Task::ExitInto`, `Task::Farewell`, `Task::Owl`; the
  talk task presses A on event texts too (Mido's), and finishes when its actor is gone with the
  box closed. A failed sandbox run now writes its trace so far.
- The sounds the owl plays (`NA_SE_EN_OWL_FLUTTER`, `NA_SE_EV_FLYING_AIR`, `NA_SE_EV_PASS_AIR`).

### Results

**Tests.** `cargo test --release --workspace` (`target/game26`, `OOT_DATA_DIR=out/data25`):
770 passed, 0 failed, 1 ignored (763 before); 7 are new. New, with their expectations from the C:
- **`oot_actors --test farewell`** (7):
  - Saria's bridge: hidden and sad at init, her fairy spawned; cue 4 kept hidden; cue 12 places
    her by the cue and fades her in (alphas 25, 51, 76, 102, 127, 153, 178, 204, 229, 255), then
    sad and opaque; cue 13 the ocarina clutched, eyes shut; cue 14 given once, then held out;
    cue 12 not after 4 sad at once.
  - The soft soil: the patch, text 0x2F, its offer for the magic bean within 40; an adult with no
    bean planted has none; a planted one is kept (not ported).
  - The owls' flags and the story: 0xFFF the Kokiri owl; the field's gone with flag 0x0C;
    Hyrule Castle's turned round; Kakariko's gone with Zelda's letter; the Lost Woods' with no
    songs.
  - The field's owl: texts 0x2064 (its chain to the question 0x2066), again 0x2065, OK 0x2067;
    `EVENTCHKINF_6F`, the owl's fanfare, one-point 8700 while he talks; his flag at the end;
    flown off; Link free, the main camera.
  - `En_Ko` child 3 at path 0's last point with the emerald, guarding within 80 of his home
    without it.
  - `Play_Init`'s layers: the field's 0, or 1 with the three stones; the forest's adult 2, or 3
    with `EVENTCHKINF_48`.
  - The exit's run (below).
- `oot_import --test pack`: the Lost Woods' draw config against the C interpreter.

**The exit run** (`Route::Farewell`, `--script farewell`, `ENTR_KOKIRI_FOREST_11`,
`deku-tree-dead`): `mido` 319, `forest_left` 1443, `bridge` 1465, `fairy_ocarina` 2256, `field`
3178, `field_intro_over` 3525, `owl_talk` 3702, `owl_flown` 4787. Texts: Mido's 0x1045, then
the bridge's 0x1011, 0x1094, 0x1012, 0x1013, 0x004A (the ocarina's) and 0x1014, then the owl's
0x2064, 0x2065, 0x2066 and 0x2067. Flags at the end: `EVENTCHKINF_1C`, `_C1`, `_A0`, `_6F`, the
Fairy Ocarina, the owl's flag; Link's ocarina in hand in the bridge's cutscene.

**The goldens.** Against 1b's 131 hashes: 130 the same bytes. `creation/shot.png` differs because
`En_Ko` child 3 now stands at his path's end, in the view down the tunnel; this was proved by
taking the path out. New: `farewell` (the trace and the end), `farewell_bridge` (frame 1800),
`farewell_ocarina` (2000), `farewell_owl` (3800); each the same bytes over two runs. Logged in
[golden/README.md](../golden/README.md): 136 hashes, 101 cases.

### Decisions

- **[ADR 0055](adr/0055-leaving-the-forest.md):** Saria's bridge and the soft soil as reached,
  the rest logged; the owl whole with one-point 8700; `Camera_SetFinishedFlag` whole;
  `Player_SetModels`' group kept apart.

### Known gaps

- **Saria's other cutscenes** and **the beans** wait for their phases (BACKLOG #28, #29).
- **The ocarina's sparkles** (`func_808526EC`'s `EffectSsKiraKira_SpawnDispersed`) aren't drawn,
  as elsewhere.
- **The owl's carrying flights** and the cutscene owls are reached only by their scenes (Lake
  Hylia, Death Mountain, later phases); `gTimeSpeed = 0` at their start waits for the clock
  (milestone 3).
- **Shadows** stay the circle stand-in at full strength.

### Fixes found while building

- **The owl's camera held forever:** `Camera_SetFinishedFlag` flagged only the main camera; the
  C also flags the active one, which ends 8700's hold (ADR 0055).
- **The talk task** never pressed A on an event text (Mido's last), and failed when its actor
  left with the box; both fixed in `oot_actors::playthrough`.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-farewell.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-farewell.bat
```

### By hand

`game-farewell.bat` (or menu 96). Space A, WASD the stick.
1. **Mido:** in the tunnel out of the meadow Mido blocks the way; talk to him: "How could you do a
   thing like that?! It's all your fault!!", and he walks off.
2. **The forest:** the Kokiri boy who guarded the Lost Woods stands aside on the hill.
3. **The bridge:** through the Lost Woods' exit, the bridge: Saria fades in, sad; she speaks,
   clutches the ocarina with her eyes shut, holds it out; Link takes it out and holds it up; "You
   got the Fairy Ocarina!"
4. **Hyrule Field:** a fade to the field's intro, the camera sweeping over it, "Hyrule Field"; the
   sky is the day's.
5. **The owl:** west, then north up the hollow below him: he speaks at once (his music), asks
   whether to hear it again; answer either way; he spreads his wings and flies off over the field.
- **What to report:** Saria's look (her face, her fade, the ocarina in her hand and Link's), the
  bridge's water, the owl's look and his head's turns while he talks, his flight, and the sounds
  (Saria's theme, the owl's music and wings).

## Milestone 3: the clock

**Answer:** done. Time passes where the rooms say: 10 a frame by day and 20 by night in Hyrule
Field; in the forest it stands. In front of the drawbridge at 17:00 the sun sets in the west:
- its lens flare and glare follow it, and hide when something stands in front of it;
- the sky and the lights turn orange, then purple;
- the field's music stops after 17:10, and the dog howls at 18:00;
- night falls and the drawbridge rises; the night's critters start after 19:00.

The exit holds. Its run is the golden `dusk`, with `dusk_sunset` and `dusk_bridge`.

The pack is still format 28 in `out/data25`, re-imported for the sun's, the moon's, the lens
flare's and the fill's bakes; builds in `target/game26`. Decisions are in
[ADR 0056](adr/0056-the-clock.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 98 to 100):
- `test-dusk.bat`: the milestone's tests (the clock, the music, the sun and the moon, the
  filters, the lens flare, the drawbridge, the Sun's Song, the depth probe, the run);
- `game-dusk.bat`: Hyrule Field at 17:00 in front of the drawbridge; `game-dusk.bat night` at
  21:00, `game-dusk.bat dawn` at 5:50;
- `sandbox-dusk.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** milestones 1 to 3 are done; 4 (Hyrule Field's props and enemies) is
next.

### What was built

**The clock** (`oot_game::clock`, ADR 0056). The time is the save's now (`dayTime`,
`skyboxTime`, `nextDayTime`, `sunsSongState`, `dogIsLost`); `gTimeSpeed` is
`EnvStatics::time_speed`.
- `Scene_CommandTimeSettings` runs from each room's header, with the skybox disables: the time
  (0xFF keeps it), the speed, the sun, and the sky's time snapped when time stands.
- `Environment_Update`'s clock:
  - the new day's countdown to the cock's crow or the dog's howl;
  - the time advanced by `gTimeSpeed`, doubled by night, held during a message, a game over, a
    sky change or a transition;
  - the sky's time following (PAL's rule), and `nightFlag`.
- `Play_Init`'s `nextDayTime` and new day (the day counts, the dog, the eggs hatching, text
  0x3066); `Environment_Init`'s resets.
- The Sun's Song's part of `Interface_Update`: 400 a frame where time passes, the next noon or
  midnight where it stands.
- Everything that reads the time reads the save's:
  - the lights;
  - the normal sky (`Skybox_Init` before the room's header, as in the C);
  - the time-based music (its rain checks, the new day's counts);
  - the scene draw configs, fed the time and `nightFlag` every frame;
  - the cutscenes' time commands (`CS_MISC` 33 and 34 too), the file select's load, the map's
    Sun's Song resets;
  - the drawbridge's layer 5 and the carrying owls.

**The sky's draws** (`oot_game::env_draw`), as bakes:
- the sun (`gSunDL` after `SETUPDL_54`) and the moon (`gMoonDL` after `SETUPDL_51`) by the time,
  on the billboard their lists load, the sun eased in cutscenes (with the C's @bug);
- the skybox filters, the lightning's flash (now drawn) and the glare, each a screen-space
  `SETUPDL_57` quad where the C fills the rectangle;
- the lens flare (`gLensFlareCircleDL`, `gLensFlareRingDL`): ten along the line from the sun,
  eased in and out, hidden by the depth under the sun.

A play state's frame starts black (`Gfx_SetupFrame`); the spikes' views keep the fog's colour.

**The depth probe** (`eng_gfx::DrawLists::depth_probe`, `eng_render::probe`):
- a frame names a pixel;
- the renderer loads its depth into a one-pixel target and reads it back after the submit;
- the app hands it to `Environment_GraphCallback` (`PlayState::environment_graph_callback`);
- the headless sandbox draws a small probe frame when its screenshots' flares need one.

**Elsewhere:**
- `Route::Dusk` (`--script dusk`, `--time 17:00`) and `Task::WaitFor`; the sandbox checks a
  route's `--time`. The trace has a `clock` field while time passes (the time, the sky's,
  `nightFlag`).
- `Inventory_ReplaceItem`; the lights' zero direction made x 1, as the C does.

### Results

**Tests.** `cargo test --release --workspace` (`target/game26`, `OOT_DATA_DIR=out/data25`): 783
passed, 0 failed, 1 ignored (770 before); 13 are new. New, with their expectations from the C:
- **`oot_actors --test clock`** (10):
  - The rooms' time settings: Hyrule Field's speed 10 with the time kept; Kokiri Forest's
    speed 0, its sky's time at 16:30 snapped to `CLOCK_TIME(17, 0) + 1`, nothing moving.
  - The clock frame by frame across 18:00: 10 by day, 20 by night, the sky's time following,
    `nightFlag` by the time; held with a message open.
  - A new day at `Play_Init`: 0x8001, the counts, the dog, the egg a Cucco and text 0x3066;
    the countdown from 0xFFFE by 0x10 to the cock's crow on the 15th frame. A night: 0, the
    dog's howl 15 frames on.
  - The field's music: the stop (`SEQCMD_STOP_SEQUENCE(SEQ_PLAYER_BGM_MAIN, 240)`) past 17:10,
    the dog past 18:00, the critters, the night's past 19:00, each a frame after its time.
  - The sun and the moon by the C's formulas at 17:00 and 23:00; the cutscene's easing, `y`
    moved twice and `z` left.
  - The skybox filters: none from `fogNear` 980; `(1000 - fogNear) * 0.02` under it; opaque
    over `SKYBOX_UNSET_1D`; the custom filter after.
  - The lens flare looking at the sun: the probe pixel (160, 115); each flare's alpha with the
    scale eased ten times; the glare; easing out when hidden; nothing when looking away.
  - The drawbridge: down by day, raised the frame after nightfall in 137 frames
    (`Math_ScaledStepToS`: 80 × 3 × 0.5 a frame); layer 5's speed and its stop at 4:00.
  - The Sun's Song: 400 a frame in the field until past 18:00 + 1, then 10 again; in the forest
    the next midnight, the fade, `Play_Init` at night.
  - The exit's run (below).
- `oot_game` `clock` (2): `NEXT_TIME_*` as clock times; the sun at noon, midnight and 18:00.
- `oot --test depth_probe` (1, on the GPU): the sky reads the far plane, the field nearer;
  `sSunScreenDepth` from both.

**The exit run** (`Route::Dusk`, `--script dusk`, `ENTR_HYRULE_FIELD_0`, `deku-tree-dead`,
`--time 17:00`): `music_faded` 68, `night` 295, `bridge_raised` 433, `night_critters` 434. The
night's step is on the first frame past 18:00, the critters' past 19:00.

**The goldens.** Against milestone 2's 136 hashes, 130 have the same bytes. The 6 that differ,
each proved by taking its cause out:
- `creation/shot.png` and `emerald/shot.png`: the room's time settings now write the save's time.
  The forest after the chain is at 12:00 (its cutscene layer's), not 10:00, and the sun's light
  falls from overhead. With the write taken out, both are back.
- `creation_nayru/shot.png`: the skybox filter covers Death Mountain's sky with its fog (layer
  4's `fogNear` 948). With the environment's draws taken out it's back; it isn't the flare's or
  the clock's.
- `farewell/shot.png` and `farewell_owl/shot.png`: time passes in Hyrule Field, so the lights
  and the sky are later's. With the clock stopped, both are back.
- `farewell/trace.json`: the new `clock` field on its 1,610 frames in the field. Without it,
  every frame is the same as with the clock stopped, and the stopped clock gives the old bytes.

New: `dusk` (the trace and the night at the castle), `dusk_sunset` (frame 150: the sunset and
its flare) and `dusk_bridge` (350: the bridge rising), each the same bytes over two runs. Logged
in [golden/README.md](../golden/README.md): 140 hashes, 104 cases.

### Decisions

- **[ADR 0056](adr/0056-the-clock.md):**
  - the time on the save, with `oot_game::clock` for the C's clock functions;
  - the sun, the moon, the filters and the flare as bakes, and fills as screen-space quads;
  - the frame on black;
  - a depth probe in the renderer for `Environment_GraphCallback`.

### Known gaps

- **The depth's frame:** the renderer reads the frame it just drew, the console the one before
  (BACKLOG #30).
- **The point lights' glows** (`Lights_GlowCheck`) aren't drawn (BACKLOG #31).
- **The sun's look:** `gSunDL` reads its strips through I8 tiles where the decomp's XML calls
  them I4; the port draws what the list says.
- `Environment_DrawCustomLensFlare` (Ganon's tower's flares) waits for its actors.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-dusk.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-dusk.bat
```

### By hand

`game-dusk.bat` (or menu 99). Space A, WASD the stick.
1. **The sunset:** walk a few steps west (left of the drawbridge as you face it). The sun sits
   low and orange over the hills with its ring and circles of flare and a faint glare. Turn
   away and back: the flare fades out and in. As the sun sinks behind the hills, the flare fades
   out before the sun is gone.
2. **Dusk:** the sky and the field go from orange to purple; the field's music stops a little
   after you start.
3. **Nightfall:** about 15 seconds in, the dog howls. The drawbridge rises with its chains and
   creaks, and the torches either side flare up.
4. **Night:** after 19:00 the crickets start; the sky is dark blue.
5. **`game-dusk.bat night`:** the moon high in the east, pale yellow; the bridge up.
6. **`game-dusk.bat dawn`:** the night's sky lightens; at 6:30 the cock crows; at 7:00 the
   field's music starts; the drawbridge comes down.
- **What to report:** the sun's shape (its strips), the flare's colours and fade, the sky's
  colours through the dusk, the moon, and when the sounds come.

## Milestone 4: Hyrule Field

**Answer:** done. Hyrule Field has its props and its enemies:
- its trees and bushes (`En_Wood02`), its bush and rock circles (`Obj_Mure2`), its signposts
  (`En_A_Obj`) and its grottos' holes (`Door_Ana`, the grottos' scenes loading with
  placeholders);
- its Peahats by day, with the flying ones' larvae (`En_Peehat`);
- its Stalchildren by night, risen round Link by the field's spawner (`En_Encount1`, `En_Skb`),
  breaking into their bones (`En_Part`, `BodyBreak`);
- the field's enemy music while a hostile enemy is within 500.

The exit holds, in two runs. By day (`Route::Field`) Link fights a grounded Peahat and goes
over the drawbridge into Castle Town's entrance. At 20:00 (`Route::FieldNight`) he fights two
Stalchildren, the owl talks at Kakariko's stairs, and he goes up into Kakariko. The night's
drawbridge is up, so the castle's way is the day's. The runs are the goldens `field` and
`field_night`, with `field_peahat` and `field_night_fight`.

The pack is still format 28 in `out/data25`, re-imported for the new actors' bakes; builds in
`target/game26`. Decisions are in [ADR 0057](adr/0057-hyrule-fields-actors.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 101 to 103):
- `test-field.bat`: the milestone's tests;
- `game-field.bat`: the field by day, 600 from a Peahat; `game-field.bat night` at 20:00 where
  the Stalchildren rise;
- `sandbox-field.bat`: both exit runs headless, their traces and screenshots.

**Status of the plan:** milestones 1 to 4 are done: Phase 7 is done.

### What was built

**The props**, each overlay whole:
- `En_Wood02`: the trees, bushes and leaves. Its spawners put five more of their kind round
  them as their places come into view; a tree bounces the sword and, bonked by a roll, drops
  from its table (or its Gold Skulltula) and sheds four leaves; a bush walked through drops once.
- `Obj_Mure2`: the circles of 9 or 12 bushes (`En_Kusa`) and 8 rocks (`En_Ishi`), out while Link
  is near, back in when he's far, the ones cut remembered.
- `En_A_Obj` (`z_en_a_keep.c`): the signposts (their text ids in the params' high byte), the
  blocks, the stump and the boulder's fragments.
- `Door_Ana`: the holes. An open one marks Link standing over its middle to fall; falling, he
  sets the return point (`RESPAWN_MODE_RETURN`, launched up on return: Player's new start mode
  4, `Player_StartMode_Grotto`) and the grotto's entrance. Hidden ones open to an explosion or
  the Song of Storms.

**The enemies**, each overlay whole:
- `En_Peehat`, grounded, flying or a larva. By day the grounded one rises within 740, hovers,
  seeks Link within 1,200 of home for 600 frames, its blades (a quad, 16 damage) digging the
  ground where their tips cut it; then home and down. By night it stays down, and a hit on its
  root lets its larvae out. Its weak point is the sphere under its body: the boomerang stuns,
  the hookshot kills, the sword hurts, fire burns. At no health it rises shrinking and explodes.
- `En_Encount1`: the spawners. In Hyrule Field by night, two Stalchildren at a time round Link,
  200 ahead and 100 the other way. Never while he stands on dirt, on a bg actor, in the air or
  swimming, nor over a water box's floor: much of the field east of the castle lies under the
  river's water box, and no Stalchild rises there. Its Leevers, Tektites and Wolfos spawn as
  placeholders.
- `En_Skb`: the Stalchild. It rises, walks at Link, swipes (4 damage), and sinks back by day or
  800 from home. A horizontal slash that doesn't kill knocks its head off (the head and the jaw
  as `En_Part`s), and headless it wanders. At no health it breaks into 16 limbs and drops from
  table 1 (rupees for the bigger ones, every tenth after ten). Its death counts it off its
  spawner.
- `En_Part` and `z_actor.c`'s `BodyBreak` (`en_part.rs`): the pieces, flying up and puffing out;
  `Effect_Ss_Dead_Sound` for the death cries.

**The draw's state** (ADR 0057). `oot_game::skelanime_std::draw_opa_pose` walks a skeleton as
`SkelAnime_DrawLimbOpa` does, with overrides that draw their own limb. The actors run it in
`draw_update` for what their draw sets: the spheres on their limbs, the blades' tips and quad,
the limbs a body break takes. Each ported enemy's breakable limbs are baked on their own
(`En_Part/<object>/<symbol>`).

**The enemy music.** The attention search notes the nearest targetable hostile enemy within 500
(`TargetCtx::bgm_enemy`). Player turns the music to `SEQ_MODE_ENEMY` with the volume by its
distance.

**Elsewhere:**
- **Player:** the roll's bonk into a tree (`En_Wood02`'s `home.rot.y`) and into a large crate
  (`Obj_Kibako2`'s `home.rot.z`), through a play request; the grotto's start mode and its exit
  from the grotto.
- **The routes:** `Route::Field` and `Route::FieldNight`. Their tasks: `Task::FightPeahat` (locked
  on, the shield up, the sword at the root within 70) and `Task::FightStalchildren` (a count, or
  every one left); `Task::Owl` takes the Kakariko owl's first text (0x206C) and fights
  Stalchildren while the owl flies off. The Mido and shop run goes round the shop's sign, now
  solid.
- `Actor`'s culling volume fields (`cullingVolumeDistance`, `Scale`, `Downward`), kept for when
  the culling is ported; the sound effects the new actors play.

### Results

**Tests.** `cargo test --release --workspace` (`target/game26`, `OOT_DATA_DIR=out/data25`): 796
passed, 0 failed, 1 ignored (783 before); 13 are new. New, in `oot_actors --test field`, with
their expectations from the C:
- The Peahats' inits by kind: scale 36 / 1000, health 6, yOffset 100, the cylinder 50 by 160;
  740 and 1,200 grounded (state 3, held at frame 3), 2,800 and 1,400 flying (untargetable); the
  larva's scale, cylinder (25 by 15, arrows and seeds only), quad and Navi id.
- By day a grounded Peahat rises within 740 (state 8), not at 760, and seeks Link; by night it
  stays down, untargetable, its yOffset 100 to -1,000 by 50 a frame.
- A hit on its root at night: three larvae, its children, `unk_2FA` 3 and 8 frames' wobble; a
  larva gone counts itself off.
- Its damage table: the nut nothing; the boomerang stunned (blue, `0x1950`, 80 frames); the
  sword a point and red; the hookshot: dying, the recoil 6 up, 14 updates to the explosion, the
  bomb on the next, gone 5 later.
- The Stalchildren's inits (scale and spheres by params, health 2, yOffset -8,000, rising
  untargetable); one by day rises, decides to sink and goes.
- A horizontal slash: a point, `breakFlags` 1, then the head and the jaw as `En_Part`s and
  `breakFlags` 3.
- The spawner: nothing by day; at night two at once, 200 ± 20 ahead and 100 ± 20 the other way,
  its children; one killed breaks into 16 parts (the 18th limb seen makes it ready: the spine is
  never taken), drops and goes, and the spawner counts 1.
- A Stalchild's death cry once (`DEADSOUND_REPEAT_MODE_OFF` counted down to 0, which plays
  nothing); with the old constant it played 40 times.
- The enemy music: none with the Peahat 700 away; `bgmEnemy` the Peahat and `SEQ_MODE_ENEMY` two
  frames after it comes within 500.
- A roll into a tree: the bonk, the tree's sway (`unk_14C` -21) and four leaves.
- An open hole: walked onto, Link falls into the grottos' scene; the return point's params
  0x04FF, at the hole.
- The exit's two runs (below).

Updated: Kokiri Forest's props test (the rock circle's 8 `En_Ishi` with the 4 placed), and the
placements test (`En_A_Obj` keeps `params & 0xFF`).

**The exit runs** (`ENTR_HYRULE_FIELD_0`, `deku-tree-dead`):
- `Route::Field` (`--script field`), from 600 east of the grounded Peahat west of the castle at
  10:00: `peahat_killed` 400, `castle_town` 1815. Link loses one heart: south over the stream,
  up the western lowland's slope (the only way up south of z 2,450), and over the drawbridge.
- `Route::FieldNight` (`--script field-night`, `--time 20:00`), from the grass north of Lon Lon
  Ranch: `stalchildren_killed` 129, `owl_talk` 613, `owl_flown` 1260, `stalchildren_gone` 1301,
  `kakariko` 1948. Over the dry stream bed where its banks are gentle; the owl takes Link within
  480 of his tree. Two more Stalchildren rise round him there; they stay frozen through the talk
  (`PLAYER_STATE1_TALKING`), and he fights them while the owl flies off. Link loses a quarter
  heart.

**The goldens.** Against milestone 3's 140 hashes, 124 have the same bytes. The new overlays
taken out with the old waypoint give all 140 back, so the enemy music and the roll's bonk change
no case. The 16 that differ, each proved by taking its cause out:
- `creation/shot.png`, `emerald/shot.png`: Kokiri Forest's signposts drawn (`En_A_Obj` out:
  back).
- `sword_chest/trace.json`: only its `actors`, +8 from frame 127: the rock circle by Mido's house
  (`Obj_Mure2` out: back).
- `playthrough/trace.json`: the same rocks' `Rand` calls (their yaws), so another bush drops:
  `bush` 1078, `deku_tree` 2217 (were 890 and 2035) (`Obj_Mure2` out: back).
- `farewell_bridge/shot.png`, `farewell_ocarina/shot.png`: the rocks nudge the farewell's walk,
  so it leaves the forest 5 frames later (`Obj_Mure2` out: back).
- `farewell/trace.json`: those 5 frames; the field's trees coming into view (`actors`); the owl's
  flight's camera, the Peahats each taking a `Rand` call every 128 frames. With `Obj_Mure2` and
  `En_Peehat` out, only the trees' `actors` remain.
- `farewell/shot.png`, `farewell_owl/shot.png`: the field's trees drawn.
- `dusk/trace.json`: the trees coming in (`actors` from frame 41); at nightfall two Stalchildren
  rise and hit Link in front of the drawbridge (frame 337). With `En_Encount1` out only the trees
  remain; the steps are the same frames.
- `dusk/shot.png`, `dusk_sunset/shot.png`, `dusk_bridge/shot.png`: the trees, and at the end the
  Stalchildren.
- `mido_shop`, `mido_shop_audio`, `new_save_deku_tree`, `new_file_deku_tree` traces: the rocks
  (`actors`, and the `switch` step 48 frames later through their `Rand` calls), and the walk round
  the shop's sign (with the overlays out and the new walk they still differ; with the old, back).

New: `field`, `field_peahat` (frame 330: the Peahat over Link), `field_night` and
`field_night_fight` (frame 80: a Stalchild hit), each the same bytes over two runs. Logged in
[golden/README.md](../golden/README.md): 146 hashes, 108 cases.

### Decisions

- **[ADR 0057](adr/0057-hyrule-fields-actors.md):**
  - the limbs' state at draw time through one skeleton walk (`draw_opa_pose`);
  - `BodyBreak` and `En_Part` whole, with each ported enemy's breakable limbs baked on their own;
  - skeletons drawn in parts where an override draws its limb;
  - no culling volume yet;
  - the enemy music's note in the attention search;
  - unported spawns as placeholders;
  - two exit runs, by day and at 20:00, where the plan had one: the drawbridge is up at night, and
    the clock takes some 2,700 frames from 10:00 to 20:00.

### Known gaps

- **No culling volume** (BACKLOG #32): spawned trees stay once in, and the Peahat digs out of view.
- **The Peahat's bomb** is a placeholder: no explosion (#33).
- **`En_Encount1`'s other kinds** spawn placeholders (#34).
- **The attention search runs in cutscenes** (#35).
- **`EffectSsDtBubble`** (kind 4 parts' bubbles) and **`Shot_Sun`** (the Sun's Song's fairy spot)
  (#36).
- **Kakariko and Castle Town** load with placeholders: Phase 8.
- **The grottos** load with placeholders, and their way back up (Link launched from the hole) is
  only exercised by hand.

### Fixes found while building

- **The enemy music** was never on: the attention search never noted `bgmEnemy` (since GAME-04).
- **The roll's bonk** only knew walls: trees and large crates now take it.
- **The owl task** knew only the forest owl's first text; it takes the Kakariko owl's too.

### Fixes after the hand test

- **Link under the field's dirt:** a decal's depth bias (`ZMODE_DEC`) is clamped to 2e-3. Its slope
  part is taken over the whole triangle, and the field's path is a few huge triangles whose near
  end is clamped at the eye (the NoN microcode): the bias pulled the path behind Link in front of
  him. 15 renders moved by a decal's pixels (logged in the golden README); the traces didn't.
- **The death cries' stutter:** `DEADSOUND_REPEAT_MODE_ON` is 2, not 0, so a cry plays once instead
  of every frame of the effect's 40 (the Stalchild's, the larva's).
- **The sky's missing frames** (`no mesh ... SKYBOX_NORMAL_SKY/1_3`): `Environment_UpdateSkybox`
  loads the first texture a call before the second, so between two entries the sky holds a pair
  the importer hadn't baked, and drew nothing for those frames. Every first texture is baked with
  every second (88 sky bakes, were 56); same pack format, re-imported.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-field.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-field.bat
```

### By hand

`game-field.bat` (or menu 102). WASD the stick, Space A, E B (the sword), Q Z (lock on), R the
shield, P the placeholders' markers.
1. **The Peahat:** it's ahead of you, a flower on the ground. It shakes, spins its blades up and
   rises with dust, then flies at you tilted forward, kicking up earth where its blades cut the
   ground. The field's music turns to the enemy theme as it nears, and back when you leave it.
2. **The fight:** hold Q to lock on (the reticle on the small body under it) and R to block its
   blades: it bounces back. When it's close, slash (E) at the small body: it flashes red. After
   six slashes it stops, rises shrinking and is gone, dropping items. There's no explosion: with
   P a marker shows where its bomb should be. Its blades take a heart if you don't block.
3. **Its home:** run more than about 1,200 from where it rose: it goes back and lands.
4. **The trees:** run and roll (Space) into a tree: Link bonks back, the tree shakes and four
   leaves drift down; sometimes an item drops. Groups of trees appear as you turn towards them.
5. **A sign:** the sign south of the drawbridge: stand in front and press Space: its text.
6. **The castle:** walk over the drawbridge by day: Castle Town's entrance loads.
7. **`game-field.bat night`:** two Stalchildren dig up out of the ground, one ahead and one
   behind, throwing earth. They walk at you and swipe. Slash one across (E): its head flies off,
   and headless it wanders. Hit it again: it falls apart into its bones, which jump and puff out,
   with its cry. Killing them, more rise; stand on a dirt path and none do.
8. **Kakariko:** from the night start, go east until the ground drops into a dry stream bed;
   follow it until both its banks slope, cross, and go north along the cliffs to the stairs'
   tree: the owl talks; pick the second answer. Go up the stairs: Kakariko's arrival plays.
9. **A grotto:** `game.bat --entrance ENTR_HYRULE_FIELD_0 --preset deku-tree-dead
   --at=-3970,-700,13860,-16384` puts you 60 from an open hole near Lake Hylia. Walk onto it:
   you drop through into a grotto (its actors placeholders). Leaving it should launch you back
   up out of the hole (Player's grotto start): report it if it doesn't.
- **What to report:** the Peahat's spin and tilt, its blades' reach and the dirt they kick up;
  the Stalchildren's rise, swipe and breaking up; the music's change near enemies; the trees'
  leaves; anything that falls through, sticks or blocks you in the field.
