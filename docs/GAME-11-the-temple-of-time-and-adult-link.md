# GAME-11: The Temple of Time, the Chamber of the Sages and adult Link

**Goal:** Phase 12 of the [roadmap](ROADMAP.md): child Link with the three spiritual stones and the
Ocarina of Time opens the Door of Time with the Song of Time, pulls the Master Sword, sees
Ganondorf ride in, wakes in the Chamber of the Sages (Rauru, the Light Medallion) and comes back to
the Temple of Time an adult, where Sheik meets him; adult Link walks out through the ruined market.
After that comes time travel in both directions: Sheik's Prelude of Light, the sword put back
(a child again) and pulled again, and the Prelude's warp. Exit (ROADMAP's outline): adult Link out
of the Temple.

**Status:** surveyed, proposed and decided (2026-10-23; the user took the breakdown as
proposed, with every recommendation: the calls' section). The phase starts from the
`ocarina-of-time` preset (GAME-10's exit review). Milestone 1 done 2026-10-23, hand-tested;
milestone 2 done 2026-10-23, hand-tested; milestone 3 done 2026-10-24, hand-tested
(their sections at the end). Phase 12 is built whole; its exit review is next.

| # | Milestone | Status |
|---|---|---|
| 1 | Adult Link and the time machine | done (2026-10-23, hand-tested); milestone 2's actors merged with it |
| 2 | The Door of Time to the Chamber of the Sages and out (the exit) | done (2026-10-23, hand-tested); milestone 3's `En_Xc` type 2 merged with it |
| 3 | Sheik's Prelude, the sword put back, the warp | done (2026-10-24, hand-tested) |

## The survey

Checked against the decomp (52a510f) and the pack (`out/data27`), not against earlier notes. Line
numbers are the decomp's. `ootx scene-info` was run on every layer of `tokinoma`, `kenjyanoma`,
`market_ruins`, `enrui`, `shrine_r` and `spot00`, and on `hiral_demo`'s layers 4 to 8.
`ootx cutscene` was run on the scripts named. Player's cutscene modes were checked against the
C's `sCueToCsActionMap`, `D_80854B18` and `D_80854E50` and the port's `cs_mode_func`. The
`ocarina-of-time` preset was read from a probe (applied, printed, then removed).

### The start: `ocarina-of-time`, read from the preset

What the exit review lists is right:
- B and the C buttons: the Kokiri Sword on B; C-Left Deku Sticks (1), C-Down the boomerang,
  C-Right the slingshot (37 seeds).
- Items: bombs (15); the Ocarina of Time in the ocarina's slot, on no button; an empty bottle;
  Zelda's Letter.
- Equipment: the Deku and Hylian Shields owned, the Hylian Shield worn; the Kokiri Tunic and
  Boots.
- Quest: the three stones; Zelda's Lullaby, Epona's Song, Saria's Song, the Sun's Song and the
  Song of Time; two pieces of heart.
- Stats: six hearts, magic level 1, 16 rupees.
- Upgrades: the silver scale, the bomb bag, the bullet bag, the stick capacity, and the
  **Goron's Bracelet** (strength 1), which the review leaves out.
- `childEquips` and `adultEquips` are both empty, as `Sram_InitNewSave` leaves them.

**`EVENTCHKINF_A7` is clear.** Phase 11 reached the Temple only through cutscene layer 11, and
entrance cutscenes don't play there (`cutsceneIndex` ≥ `CS_INDEX_0`). So the first ordinary
entrance, `ENTR_TEMPLE_OF_TIME_0`, plays `gTempleOfTimeIntroCs`. The door's flags are all clear:
0x4B, 0x4F, 0x45, 0xC5, 0x55.

### The scenes and their layers

Layers 2 and 3 of every scene here have header 0, so they are layer 0's set. The adult's world is
mostly other scenes: the entrance table's rows +2 and +3 name the ruins
(`entrance_table.h:85-88, 235-238, 458-461`). Examples: `ENTR_MARKET_DAY_0` leads to
`SCENE_MARKET_RUINS` for an adult, and `ENTR_HYRULE_CASTLE_0` to `SCENE_OUTSIDE_GANONS_CASTLE`
(Phase 19). The scene layer is chosen in `z_play.c:409-441`, and the port does the same
(`play_scene.rs:451-476`).

| Scene | Layer | Cutscene | Who meets it |
|---|---|---|---|
| `tokinoma` (Temple of Time) | 0 to 3 (one set) | | Child and adult. Room 1 (the hall): `En_Okarina_Tag` 0x13FF (type 4, the Door of Time's song spot), `Demo_Kankyo` 0x000D (the door) and 0x000E (the light plane), `Bg_Toki_Hikari` 0 and 1, `Bg_Mjin` 1 (the Light Medallion's pad), `En_Wonder_Talk`, `Demo_Effect` 0x13 to 0x15 (the stones' places). Room 0 (behind the door): `Bg_Toki_Swd` 0xFFFF (the pedestal), `En_Xc` 9 (Sheik), `Demo_6K` 0x10, `Bg_Spot02_Objects` 4, `Demo_Kankyo` 1 (blue rain), `Demo_Effect` 0xFF0F (the time warp). The rooms are joined by `En_Holl` 0x013F |
| | 4 | `gTempleOfTimeFirstAdultCs` | Sheik's awakening script with `En_Xc` 1. In play, `En_Xc` 9 starts this script on the adult layer (`z_en_xc.c:2206-2211`), so layer 4 is the map select's (to check) |
| | 5 | `gTempleOfTimeCreditsCs` | The credits (Phase 19): `End_Title`, the lens flare (BACKLOG #55) |
| | 6 | `gTempleOfTimePreludeCs` | The Prelude's script with `En_Xc` 2. In play `En_Xc` 9 starts it too (`:2212-2217`) |
| | 7 to 10 | the door opening, the stones restored (all "Unused") | Nobody: a development path (`z_demo.c:1220-1254`'s comment) |
| | 11 | `gTempleOfTimeSongOfTimeCs` | Phase 11 (done) |
| | 12 to 14 | Sheik's reveal, the light arrows | Phase 19 (`En_Zl2`, `En_Xc` 4) |
| `kenjyanoma` (Chamber of the Sages) | 0 to 3 (one set) | | The medallions' scenes by `chamberCutsceneNum` (later phases): `Demo_Sa`, `Demo_Du`, `Demo_Im`, `En_Ru2`, `En_Nb` (each type 0), `Door_Warp1` -2 (`WARP_BLUE_CRYSTAL`), `Demo_Effect` 0x12 |
| | 4 | `gChamberOfSagesLightMedallionCs` | **This phase:** `En_Rl` 0 (Rauru), `En_River_Sound` 0x10 |
| | 5 | `gChamberOfSagesRauruFinalMsgCs` | After both the Shadow and the Spirit Medallions (`z_demo.c:1256-1282`): Phase 18 |
| | 6 | `gChamberOfSagesSealOpeningCs` | Phase 19 |
| `hiral_demo` (the cutscene map) | 7 | `gCutsceneMapGanondorfFromMasterSwordCs` | **This phase:** `En_Viewer` 0x07FF (the giant Ganondorf, ported) |
| `shrine_r` (Temple of Time exterior, ruins) | 0 to 3 | | An adult. Prerendered, with two fixed cameras: 4 gossip stones (`En_Gs`), `Bg_Spot16_Doughnut` (the mountain's smoke ring). All ported |
| `market_ruins` | 0 to 3 | | An adult. Prerendered pivot camera: 8 ReDeads (`En_Rd` 0x8100, 0x8102, ported). Exits: 1 the market entrance, 2 the castle (Ganon's for an adult), 3 the Temple's outside |
| `enrui` (market entrance, ruins) | 0 to 3 | | An adult: `En_Weather_Tag` 0x0200, an `En_Door` (the guard house). Exit 2 is `ENTR_HYRULE_FIELD_7` |
| `spot00` (Hyrule Field) | 2, 3 | | An adult: `Bg_Spot00_Break` ×4 (the broken drawbridge, at the field's spawn 7), `En_Po_Field` ×10, `Obj_Hamishi`, `En_Weather_Tag` ×2. No owls, Peahats or drawbridge. Coverage gives it to Phase 13 |

**Corrections to `scripts/coverage.py` (proposed):**
- **`ACTORS`:** `"Bg_Toki_Swd": 12`, `"Demo_6K": 12`, `"En_Xc": 12`. Today they count as Phase 8's
  three unported actors because the child's Temple layers place them, behind the door. This
  moves them to this phase and leaves Phase 8 complete.
- **`kenjyanoma`:** `(14, 14, 12, {5: 18, 6: 19})`. Layers 0 to 3 are the medallions' scenes (the
  first one is the Forest's), and 5 is Rauru's message after the Shadow and Spirit Medallions.
  Add `ACTORS` `"En_Ru2": 16` and `"En_Nb": 18` (the Water Temple and the Spirit Temple).
- **`spot04` and the Kokiri houses' adult layers go to 14**, the outline's "Kokiri Forest as an
  adult". This moves `Obj_Mure3` (the tower of rupees) to 14.
- **`market_day`, `market_night`, `entra`, `entra_n`, `shrine`, `shrine_n`:** adult layers `None`.
  The entrance table's adult rows lead to the ruins, so play never loads these layers.
- **Phase 12's row then reads** `Bg_Toki_Swd`, `Demo_6K`, `En_Rl`, `En_Xc`, plus
  `Bg_Spot00_Break` if call 5 brings it in.

### The route

1. **To the Temple:** Hyrule Field, then the market, then `shrine` (all ported). Entering at
   `ENTR_TEMPLE_OF_TIME_0` the first time plays **`gTempleOfTimeIntroCs`** and sets
   `EVENTCHKINF_A7` (`z_demo.c:129`). The title card is from `CS_MISC` 0xF.
2. **The Door of Time.** `En_Okarina_Tag` type 4 (ported) is at the altar, at (-1, -40, 1384).
   - Within 90 + the interact range across and 80 up or down, it listens with
     `OCARINA_ACTION_CHECK_TIME` (`z_en_okarina_tag.c:212-239`).
   - Played right, it starts **`gDoorOfTimeOpeningCs`** (3,000 frames; `:271-274`). Link puts the
     ocarina away (modes 26, 27, 8). `Demo_Effect` 0x13 to 0x15 bring the stones on cue 48.
   - `Demo_Kankyo` 0xD slides the door open on cutscene flag 2 and sets
     **`EVENTCHKINF_OPENED_DOOR_OF_TIME`** 0x4B as it finishes (`z_demo_kankyo.c:443`). After
     that, its child `Door_Toki` (the door's collision) drops its DynaPoly (`z_door_toki.c:58`).
     `Bg_Toki_Hikari` 1 (the light) goes. `CS_MISC_STOP_CUTSCENE` comes at frame 790; there is
     no terminator.
   - Ported already: the song spot, `Demo_Kankyo`, `Demo_Effect`, `Bg_Toki_Hikari`. Missing:
     **`Door_Toki`** (63 lines; a placeholder now, so the closed door may have no collision:
     to check).
3. **The Master Sword.** Through the `En_Holl` into room 0.
   - **The reveal.** Facing `Bg_Toki_Swd` within 800, outside a cutscene layer and without
     `EVENTCHKINF_REVEALED_MASTER_SWORD` 0x4F, it sets 0x4F and plays
     **`gRevealMasterSwordCs`** (`z_bg_toki_swd.c:110-115`). An actor moves from Link to the
     sword on cue 62, `npcActions[8]` (Navi, by the texts' colour: to confirm). The texts are 0x70E6 ("…isn't that...?!"), 0x70E7 and 0x70E8 ("The Master Sword!!").
   - **The pull.** A child (or an adult with `EVENTCHKINF_55`) facing it is offered it to carry
     (`Actor_OfferCarry`). Player's `Player_ActionHandler_2` and `func_8083A0F4` parent it and wait
     for the cutscene. Then the child gets **`Item_Give(ITEM_SWORD_MASTER)`** and
     **`gPullMasterSwordCs`**, with `NA_BGM_MASTER_SWORD` (`:116-128`).
   - **The pull's script** (425 frames): Player's mode 18, whose start `func_808519EC` puts him on
     the pedestal (ported). Its update **`func_80851A50`** (missing) parents the sword on the
     child's frame 87 or the adult's frame 70. It also swaps the left hand (`gPlayerLeftHandBgsDLs`
     or `...ClosedDLs`) and plays the sounds (`z_player.c:15538-15569`).
   - `CS_MISC_SET_CSFLAG_1` at frame 110 dims the temple: `Bg_Toki_Swd` steps `drawParams[0]`
     to 255, and `SDC_TEMPLE_OF_TIME` reads it.
   - **Terminator 8** (`CS_DEST_TEMPLE_OF_TIME_FROM_MASTER_SWORD`, `z_demo.c:688-712`): the first
     time it sets **`EVENTCHKINF_OBTAINED_MASTER_SWORD`** 0x45 and goes to `ENTR_CUTSCENE_MAP_0`
     with `CS_INDEX_3`. Farore's Wind and the top respawn are cleared. Ported except the age.
4. **Ganondorf** (`hiral_demo` layer 7, **`gCutsceneMapGanondorfFromMasterSwordCs`**, 1,250
   frames): `En_Viewer` type 7 (ported), text 0x700A ("Geh heh heh! Excellent work!…").
   **Terminator 25** (`CS_DEST_CHAMBER_OF_SAGES_LIGHT_MEDALLION`, `:820-826`): **`linkAgeOnLoad =
   LINK_AGE_ADULT`**, then `ENTR_CHAMBER_OF_THE_SAGES_0` with `CS_INDEX_0` and a white fade.
5. **The Chamber of the Sages** (layer 4, **`gChamberOfSagesLightMedallionCs`**, 3,048 frames).
   - Link arrives by spawn 0's start mode 2, the blue warp's (ported). He stands up as an adult:
     modes 8, 23 and 29, all ported.
   - **Rauru** (`En_Rl` type 0, cue 39) talks: 0x704E ("Wake up… the chosen one"), 0x704F ("I am
     Rauru…"), Navi's 0x7050 ("You're big now!!"), and 0x7051 with its choice (0x7053, or 0x7052
     back to 0x7051).
   - Then Rauru spawns `Demo_Effect` 0xE (the medallion) and calls
     **`Item_Give(ITEM_MEDALLION_LIGHT)`** (`z_en_rl.c:132-149`). Texts 0x0040 ("You received
     the Light Medallion!") and 0x70E9 ("Find the other Sages…") follow.
   - **Terminator 18** (`CS_DEST_TEMPLE_OF_TIME_AFTER_LIGHT_MEDALLION`, `:775-783`) goes to
     `ENTR_TEMPLE_OF_TIME_4` (spawn 4, room 0, the pedestal's step) with a black fade. The debug
     ROM also sets 0x4F there.
   - The chamber's draw config **`SDC_CHAMBER_OF_THE_SAGES`** (27 lines) isn't ported, nor is
     `chamberCutsceneNum` (`cutscene.rs:1076`).
6. **The awakening.** The Temple now loads as an adult (layer 2, the same set as layer 0).
   - **`En_Xc` 9** (`EnXc_InitTempleOfTime`, `z_en_xc.c:2205-2226`, killed for a child) sees no
     **`EVENTCHKINF_C5`**. It sets the flag and plays **`gTempleOfTimeFirstAdultCs`** (2,249
     frames), running type 1's actions (`func_80B3EBF0`).
   - Navi speaks first: 0x7054 ("…we're back in the Temple of Time…"). Then Sheik: 0x7056 ("I've
     been waiting for you, Hero of Time..."), 0x7057, and 0x7077 to 0x707A ("I am Sheik…",
     "…the Forest Temple…").
   - `Demo_6K` 0x10 (a light ball on cue channel 2, shown while the cue is id 3) and
     `Bg_Spot02_Objects` 4 (the light beam, ported) play their part.
   - Player runs 14 modes. All are ported except mode 30's update, **`func_80851D80`** (the sword
     drawn on frame 6, `z_player.c:15616-15624`).
   - `CS_MISC_STOP_CUTSCENE` comes at frame 1,020; there is no terminator.
7. **Out** (the exit). Room 1, then exit 1 (`ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1`), which for an
   adult leads to `shrine_r`. Then `market_ruins` (the ReDeads), `enrui`, and the field at
   `ENTR_HYRULE_FIELD_7` (spawn 7, by the broken drawbridge). Every actor on this way is ported
   except `Bg_Spot00_Break`.
   - On later visits without `EVENTCHKINF_55`, Sheik stands in front of the pedestal
     (`func_80B3C9EC`, `EnXc_BlockingPedestalAction`).
   - Her texts there: 0x700F ("…another skill... Head to Kakariko Village!"), or 0x7010 with the
     hookshot.
8. **The Prelude of Light** (`z_en_xc.c:2212-2217`).
   - It needs **`EVENTCHKINF_48`**, which the Forest Temple's blue warp sets
     (`z_door_warp1.c:724-725`): Phase 14's. With 0x48 and no **`EVENTCHKINF_55`**, `En_Xc` 9 sets
     0x55 and gives **`ITEM_SONG_PRELUDE`**.
   - It then plays **`gTempleOfTimePreludeCs`** (2,516 frames), running type 2's actions (SHEIK_ACTION_30
     on).
   - Texts: 0x7074, 0x708B ("…return the Master Sword to the Pedestal of Time…"), 0x708C. Then
     the teaching (ocarina actions 7 and 0x14, played back to 0x88B), 0x0078 ("You have learned
     the Prelude of Light!") and 0x7075.
   - Player's modes are all ported. Sheik's harp is drawn.
   - **The real game's order holds here:** going back to a child needs the Forest Temple. The
     run is a debug start with 0x48.
9. **Back to a child.**
   - An adult with 0x55 puts the sword back (`z_bg_toki_swd.c:116-128`). The pedestal shows it
     again once he lets go (`:148-155`).
   - That plays **`gPlaceMasterSwordCs`** (368 frames, mode 18 again). Its **terminator 8**, with
     0x45 set, sets **`linkAgeOnLoad`** to the other age outside a cutscene layer. Then it goes to
     **`ENTR_TEMPLE_OF_TIME_2`** with a white fill (`z_demo.c:699-711`).
   - Spawn 2's params 0x0100 are **`PLAYER_START_MODE_TIME_TRAVEL`** (missing in the port).
     `Player_StartMode_TimeTravel` stands Link on the pedestal. For an adult it puts the sword in
     hand. Then **`Player_Action_TimeTravelEnd`** plays `timeTravelEndAnim` after 20 frames
     (`z_player.c:10576-10599, 13945-13985`).
   - A child pulling again plays `gPullMasterSwordCs`. Terminator 8 then makes him an adult at the
     same spawn.
10. **The Prelude's warp.** Player's warp song action is ported (`player.rs:10995-11025`):
    `sWarpSongEntrances`, mode 8, the `Demo_Kankyo` warp-out, then `Environment_WarpSongLeave`.
    The message box's "warp?" choice is ported too. The Prelude lands at
    **`ENTR_TEMPLE_OF_TIME_7`** (spawn 7, room 1, on `Bg_Mjin`'s pad), with start mode 5's
    arrival and the warp-in sparkles (ported). Still open, BACKLOG #26: Player isn't hidden
    through the white-out, and sub-timers aren't cut.

**The age switch** (`z_play.c:300-303`, `z_player.c:12362`, `z_scene.c:277`):
- A terminator sets `play->linkAgeOnLoad`.
- `Play_Destroy` sees it differ from `linkAge`. While the old age is still current, it calls
  **`Inventory_SwapAgeEquipment`**, which saves this age's B, C buttons and equipment and loads
  the other's (the first time as an adult: the Master Sword, the Hylian Shield, the Kokiri
  Tunic). Then it calls `Player_SetEquipmentData`.
- `Player_Destroy` then sets `linkAge = linkAgeOnLoad`. The next `Play_Init` picks the layer by
  the new age.
- No `z_demo.c` case swaps itself. The port logs every `linkAgeOnLoad` (`cutscene.rs:1048`,
  thirteen destinations); this phase's are 8 and 25.

### Adult Link: what Player has and what's missing

The spikes ported Player for both ages, and the import reads the C's tables for both. So most of
adult Link is already there:
- **Draw rules and meshes.** `PlayerRules::load` reads `z_player_lib.c`'s model groups,
  `gPlayerModelTypes`, `sTunicColors` and the faces (`oot_import/src/player.rs:26-112`).
  `import_link` bakes both ages: every model group × fists × the four shields × sword × shield
  held, plus the boomerang thrown and first person (`oot_import/src/pack.rs:662-760`). The pack
  stores them as `player/adult/...` and `player/child/...`.
- **`sAgeProperties`, the animations and the REGs:** both ages (`tables.rs:188-238`,
  `data.rs:53-106`).
- **Already age-generic:** the voice and footsteps by age, the ledge and climb heights, the
  door animations, the crawlspace, the bow's aim and ammo, the bottle and bow string per age,
  `Player_DrawGetItem`, every sword's length and damage, the Hylian Shield on the back, the
  shield's collider, and the Kokiri Forest adult layer case.
- **The scene layer by age** and the entrance table's four rows (`play_scene.rs:451-476`).
- **The pause menu's item page** greys items by age (`kaleido/item.rs`).
- **The save** keeps `adult`, `child_equips` and `adult_equips` in SRAM (`sram.rs:305-337`);
  `Sram_OpenSave`'s adult entrance and its Master Sword re-give are there too.
- **`--adult`** makes a new save (`Sram_InitNewSave`): nothing on B, nothing worn. The scene-only
  sandbox uses `Sram_InitDebugSave` instead (the Master Sword, the bow, the Hylian Shield).
  Adult starts in this phase come from presets.

| Missing | C | Lines |
|---|---|---|
| **The age switch:** `Inventory_SwapAgeEquipment`; `linkAgeOnLoad` (the scene command, `Play_Destroy`'s swap, `Player_Destroy`, the 13 destinations' sets) | `z_parameter.c:1210-1317`; `z_play.c:300-303`, `z_player.c:12362`, `z_scene.c:277`, `z_demo.c` | 108 + 20 |
| **The pedestal in Player:** `Player_ActionHandler_2`'s adult `Bg_Toki_Swd` branch (`PLAYER_IA_SWORD_CS`), noted at `player.rs:11210-11214`; the child's is ported (`func_8083A0F4`) | `z_player.c:7414-7428` | 15 |
| **Time travel's arrival:** `Player_PutSwordInHand`, `Player_StartMode_TimeTravel`, `Player_Action_TimeTravelEnd` | `z_player.c:10557-10599, 13945-13985` | 83 |
| **Cutscene modes 18 and 30's updates:** `func_80851A50` (with `D_808551A4`, `D_808551AC`), `func_80851D80`; and a left-hand override in the `Loadout` | `z_player.c:15524-15569, 15616-15624` | 55 |
| **The adult's draw details:** the gauntlets' plates (strength 2 and up), the iron and hover boots' lists, the tunic's colour at draw time (now baked as tunic 0's), the broken knife's hand | `z_player_lib.c:1079-1087, 1125-1157, 1364-1365` | 40 |
| **The pause menu's equipment page** (now a stand-in that equips the first owned piece the age can wear, `save.rs:1913-1935`) and Link drawn in it | `z_kaleido_equipment.c` (715), `z_player_lib.c:1890-2116` (227) | 942 |
| Later phases, for reference: the endgame's cutscene modes (`func_80852298`, `_80852608`, `_80852648`, `_8085283C`, `_808528C8`), the hookshot's reticle, the boots' data, the hammer's quake | | 46 + 37 + 43 |

The equipment page's stand-in already does what the swap needs here: a first adult has the
Master Sword and Hylian Shield put on.

### The adult's placeholders chosen at init (Phases 6 to 11)

98 overlays check the age; 66 of them are ported. The sweep compared each with its module.

**Already whole:**
- Generic actors: `En_Elf` (Navi's adult texts), `En_Item00`, `En_Box` (the adult's opening),
  `En_Door`, `En_Kanban`.
- The Temple's and the warps' actors: `Door_Warp1`'s `WARP_SAGES` and `WARP_DESTINATION`,
  `Demo_Kankyo`, `Demo_Effect` (the adult's jewel and the medallions), `Bg_Toki_Hikari`,
  `Bg_Spot02_Objects` 4.
- Adult NPCs: `En_Rd`, `En_Ani`, `En_Fu`, `En_Ta`, `En_Toryo`, `En_Hy`, `En_Niw_Lady`,
  `En_Kakasi`, `En_Kakasi3`, `En_Skj` 63, `En_Du`, `En_Gm`.
- Set pieces: `Bg_Spot18_Obj`, `_Shutter`, `Bg_Spot07_Taki` (the frozen falls),
  `Item_Etcetera`, `En_GirlA`, `En_Horse_Normal`, `Bg_Dy_Yoseizo`.
- The scene draw configs' adult branches.

**What this phase must make whole** (on its route): Player's pedestal branch, `Door_Toki`, the
age's destinations, `chamberCutsceneNum`, `SDC_CHAMBER_OF_THE_SAGES`. These are framework and
unported pieces rather than adult branches of ported actors.

**What belongs to later phases** (each with its own phase, by the outline):

| Phase | Actor: the adult's part | How the port has it | C |
|---|---|---|---|
| 13 | `En_In`: Ingo's race and rental | placeholder at init (`en_in.rs:510-516`) | ~350 |
| 13 | `En_Ma2`: adult Malon | placeholder (`en_ma2.rs:105-108`) | ~300 |
| 14 | `Demo_Sa` 0, 1: the Forest Medallion | logged, draws nothing (`demo_sa.rs:217`) | 135 |
| 14 | `Door_Warp1`: the adult dungeons' warp, the clear flag's, `WARP_UNK_7`; `WARP_BLUE_CRYSTAL` (the Chamber's layers 0 to 3) | logged (`door_warp1.rs:294-306`) | 270 + 130 |
| 14 to 18 | `Obj_Bean`: the adult's beanstalk lift (BACKLOG #29) | logged, stands (`obj_bean.rs:104`) | ~325 |
| 15 | `En_Go2` 1, 2, 3: Link the Goron, Biggoron, the Fire Temple's Gorons (#72) | placeholders | ~530 |
| 15 | `Demo_Du` 0: the Fire Medallion (#83) | placeholder (`demo_du.rs:293`) | 137 |
| 16 | `En_Kz`'s ice (`Bg_Ice_Shelter`), `Bg_Spot06_Objects`' lock and drained lake, `Obj_Ice_Poly` (#75) | placeholders | 8 + 515 + 130 + 220 |
| 17 | `Demo_Im` 0, 1: the Shadow Medallion (#53, which says "Phase 12": it's 17) | logged, hidden | ~128 |
| 17 | `En_Ossan`: Kakariko's potion shop, the adult bazaar (#100) | placeholder | ~70 |
| 19 | `Demo_Du` 2, 3, `Demo_Sa` 2 to 4, `Demo_Im` 2, 6, `En_Viewer` 9's draw, `Door_Warp1`'s purple crystal | logged or placeholders | 284 + 264 + 217 + 30 + 50 |
| 20 | `En_Ko`: Fado's saw trade | not ported | ~57 |

**Killed for the other age in the C, and the port matches:** `Bg_Spot00_Hanebasi`,
`Bg_Spot01_Idohashira`, `Bg_Spot02_Objects` 1, `Bg_Spot05_Soko` 0, `Bg_Spot06_Objects`' ice block,
`En_Cow` (Link's house), `En_Encount2`, `En_Heishi4`, `En_Ms`, `En_Okarina_Tag` 2, `En_Tk`,
`En_Zo` 8, `En_Ko` (by `rot.z`), `En_Go2`'s bazaar Gorons, and the spawn checks of `En_Ma1`,
`En_Daiku_Kakariko`, `En_Du`, `En_Toryo`, `En_Hy`, `En_Md`, `En_Ta`, `En_Horse_Normal`.

**Docs out of date** (to fix at milestone 1's start):
- BACKLOG #37 lists `Bg_Spot02_Objects` 4 as a placeholder, but it's ported whole.
- BACKLOG #53 should say Phase 17.
- BACKLOG #55 (the custom lens flare) is layer 5's, the credits: Phase 19, not 12.
- GAME-10's "What's left" lists `Bg_Spot07_Taki`'s frozen falls, but they're whole.
- `demo_kankyo.rs:20` says `Door_Toki` waits for "Phase 8".
- `obj_timeblock.rs:251` calls `Demo_Effect` unported.

### What's already ported and reused

- **The Temple's hall:** `En_Okarina_Tag` (type 4), `Demo_Kankyo` (the door, the light plane, the
  blue rain, the warp sparkles), `Demo_Effect` (the stones, the time warp 0xF, the medallion 0xE,
  the Chamber's 0x12), `Bg_Toki_Hikari`, `Bg_Mjin`, `Bg_Spot02_Objects` 4, `En_Wonder_Talk`,
  `SDC_TEMPLE_OF_TIME`.
- **The cutscene map's Ganondorf:** `En_Viewer` 7.
- **The ruins:** `En_Rd`, `En_Gs`, `Bg_Spot16_Doughnut`, `En_Weather_Tag`, `En_Door`.
- **The cutscene system:** destinations 8, 25 and 18 (all but the age), the entrance intro
  table, `CS_MISC` 0xA (cutscene flag 1), 0xC, 0xD and 0xF, the text lists with choices, the
  ocarina's teaching and playback in a script (Phase 11's Song of Time).
- **Player:** the cutscene mode tables and all their typed handlers, every named function this
  route's scripts cue but two (`func_80851A50`, `func_80851D80`), the child's pedestal lift, the pedestal's do-action, the blue warp's
  arrival (the Chamber's spawn), the warp songs' action and arrival, the ocarina, `Item_Give` for
  the Master Sword, the Light Medallion and the Prelude.
- **The message box:** the warp songs' "warp?" choice, `disableWarpSongs` and the restrictions.

### The systems

| System | C | The port has | Missing |
|---|---|---|---|
| **The age** | above | `adult`, both equips in the save and SRAM, the layer and entrance by age | the swap, `linkAgeOnLoad` (128) |
| **Time travel** | above | the pull's start, the cutscene destinations | the arrival (83), modes 18 and 30 (55), the adult's pedestal (15), `Bg_Toki_Swd` (191) |
| **Adult Link's look** | `z_player_lib.c` | everything the import bakes | the gauntlets, boots, tunic colour, the broken knife's hand (40) |
| **The Chamber** | `SDC_CHAMBER_OF_THE_SAGES` (27), `chamberCutsceneNum` (3) | the scene | both |
| **The warp songs** (BACKLOG #26) | `player->actor.draw = NULL` and the sub-timers | the action, the arrival, the box | Player's draw switch; `Interface_SetSubTimerToFinalSecond` (a no-op) |
| **The equipment page** | 942 | a stand-in | the page (call 3) |

### Things to check (for the user, or at a milestone's start)

- **The closed Door of Time.** With `Door_Toki` a placeholder, its collision isn't there. Can a
  child walk through the closed door in the port now? BACKLOG #37 says the room loads only
  through the `En_Holl`.
- **Adult Link in the ruins' prerendered scenes** (`shrine_r`, `enrui`, `market_ruins`): these
  are the first prerendered places an adult walks, with their fixed and pivot cameras.
- **Who loads the Temple's layers 4 and 6** in play. The scripts are started by `En_Xc` 9 on the
  adult layer; the layers look like the map select's. To confirm from the cutscene indexes
  before the runs.
- **The pull's dimming:** that `SDC_TEMPLE_OF_TIME` reads `roomCtx.drawParams[0]` as the C does.
- **The ReDeads** grab an adult (`en_rd.rs:830`), and their scream freezes him. A run never
  fights them: they are the hand test's, or the walk-out run goes round them. Their range to
  check.
- **`ENTR_TEMPLE_OF_TIME_4`**'s spawn (0, 70, 46) is on the pedestal's step, facing -0x0E39.
  Sheik's script is placed from it.
- **The first adult's equips** after the swap: the stand-in's choice should match the C's
  `Inventory_SwapAgeEquipment` (the Master Sword on B, the Hylian Shield, the Kokiri Tunic and
  Boots, the C buttons empty: no adult items yet).

### Sizes

| Part | Lines of C (about) |
|---|---|
| Adult Link and the time machine: the age switch, the pedestal in Player, the arrival, modes 18 and 30, the left-hand override, the adult's draw details; `Bg_Toki_Swd` (its three scripts are in the pack already), `Door_Toki` | 128 + 15 + 83 + 55 + 40 + 191 + 63 = 575 (+ 274 lines of script data) |
| The Door of Time to the Chamber and out: `En_Rl` whole, `SDC_CHAMBER_OF_THE_SAGES`, `chamberCutsceneNum`; `En_Xc`'s shared code, type 1, type 9 and the pedestal block; `Demo_6K` types 14 to 19 with its shared code; `Bg_Spot00_Break` (call 5) | 422 + 27 + 3 + 700 + 260 + 80 = 1,490 |
| Sheik's Prelude and the warp: `En_Xc` type 2 and the harp's draws, BACKLOG #26 | 155 + 45 + 10 = 210 |
| The equipment page and Link in it (call 3) | 942 |
| Not this phase: the rest of `En_Xc` (the Minuet, Bolero, Serenade, Nocturne and the reveal: 14 to 19) and of `Demo_6K` (19); `En_Po_Field` (13); `Obj_Mure3` (14) | (1,800 + 560 + 1,018 + 205) |

**About 2,300 lines** (3,200 with the equipment page). This is the smallest phase since the cutscene
phase, because the spikes ported Player for both ages and the import bakes the adult. `En_Xc`
(2,494) is the largest overlay here and is mostly later phases' Sheik.

## The proposal

Three milestones in the story's order, with the time machine first. The exit needs the age
switch, the pedestal, the Chamber and Sheik's awakening. So milestone 1 builds the age switch and
both directions of the pedestal on a debug start. Milestone 2 plays the story from
`ocarina-of-time` to adult Link outside the Temple (the exit). Milestone 3 does the Prelude and
going back.

| # | Milestone | What | Run (short debug starts; no fights) |
|---|---|---|---|
| 1 | Adult Link and the time machine | The age switch (`Inventory_SwapAgeEquipment`, `linkAgeOnLoad` and its 13 destinations, `Play_Destroy`'s swap, `Player_Destroy`); `Bg_Toki_Swd` whole; `Door_Toki`; Player's adult pedestal branch, `Player_PutSwordInHand`, start mode 1 and `Player_Action_TimeTravelEnd`, `func_80851A50`, `func_80851D80`, the left-hand override; the adult's draw details (gauntlets, boots, tunic colour at draw time, the broken knife); the stale docs fixed; the coverage corrections | A debug start in room 0 with the door open, the sword revealed and `EVENTCHKINF_OBTAINED_MASTER_SWORD`: the child pulls (`gPullMasterSwordCs`), terminator 8, the equips swapped, the adult arrives on the pedestal (`TimeTravelEnd`). Then, with `EVENTCHKINF_55` added, he puts it back (`gPlaceMasterSwordCs`) and arrives a child with the Kokiri Sword again |
| 2 | The Door of Time to the Chamber of the Sages and out (**the exit**) | `En_Rl` whole; `SDC_CHAMBER_OF_THE_SAGES`, `chamberCutsceneNum`; `En_Xc` types 9 and 1 (the awakening) and the pedestal block, its other types placeholders chosen at init; `Demo_6K` types 14 to 19, the others placeholders; `Bg_Spot00_Break` (call 5) | From `ocarina-of-time` at the Temple's door: the intro, the Song of Time at the altar, the door opening. The reveal and the first pull, Ganondorf, the Chamber (Rauru, the Light Medallion), back as an adult, Sheik's awakening. From the end of the awakening, a debug start out through room 1 to `shrine_r`: **the exit**. The ruins on to the field (the ReDeads gone round, or the hand test's) |
| 3 | Sheik's Prelude, the sword put back, the warp | `En_Xc` type 2 (the Prelude) and the harp's draws; BACKLOG #26 (Player hidden through the warp's white-out; the sub-timer cut) | A debug start: the adult with the Forest Medallion's `EVENTCHKINF_48` entering the Temple. The Prelude taught (0x55); the sword put back (a child), pulled again (an adult); the Prelude played elsewhere warps to `ENTR_TEMPLE_OF_TIME_7` |

The scope rule is the same as before: every actor whole, except the parts another age's or another
phase's init chooses (their init's checks, then a placeholder). Framework work (the age switch,
Player, the save) stays in the main session. The helpers can take `En_Xc` and `Demo_6K` with
`En_Rl`.

Presets, chained as before:
- `master-sword`: the door open, the sword revealed and obtained. Milestone 1's debug start.
- `light-medallion`: after the Chamber. Its end is the adult's first save.
- `adult-awake`: after Sheik's script. The exit's start and **Phase 13's start**.
- `prelude`: `adult-awake` with 0x48 and the Forest Medallion. A debug start.
- `child-again`: after the sword is put back.

## The user's calls

2026-10-23: the user took the breakdown as proposed, with the recommendation on every call
below:
- **The split:** the three milestones in this order.
- **The Prelude and going back to a child:** in, from a debug start with `EVENTCHKINF_48`.
- **The equipment page:** left for Phase 20; the stand-in stays.
- **The adult placeholders from Phases 6 to 11:** each with its own phase.
- **`Bg_Spot00_Break`:** in, with milestone 2.
- **`En_Xc` and `Demo_6K`:** split by type, the other types placeholders chosen at init.
- **The coverage corrections:** applied with milestone 1's coverage refresh.
- **BACKLOG:** #37 and #57 close with milestone 1, #26 with milestone 3; #53 and #55 re-filed
  (Phases 17 and 19) at milestone 1's start, with the other stale docs.

The milestones are built one after another; the hand tests come as `game-*.bat` debug starts.

The calls as proposed:

1. **The split and its order** as proposed: three milestones, the time machine first, the exit
   at 2, the Prelude and the way back at 3. Recommended. Or: fold 3 into 2 (it's small) and keep
   two.
2. **The Prelude and going back a child before the Forest Temple.** The C gates both on
   `EVENTCHKINF_48` (the Forest Temple's warp) and 0x55 (the Prelude). Recommended: in this phase
   from a debug start that sets 0x48, as the outline asks (time travel back and forth). The
   story's chain keeps the real order: `adult-awake` goes on to Phase 13.
3. **The pause menu's equipment page and Link drawn in it** (942 lines, BACKLOG #4's last part).
   The stand-in already puts the right things on after the swap. Recommended: Phase 20 (the pause
   menu in full), as the outline has it. Or: here, as a fourth milestone, since adult Link's
   equipment is this phase's subject.
4. **The adult's placeholders from Phases 6 to 11:** none sits on this phase's route.
   Recommended: each with its own phase (the table above). Or: bring in the small ones an adult
   can reach from here with what he has (`En_Ossan`'s potion shop and bazaar, about 70; `En_Ko`'s
   saw trade, 57).
5. **`Bg_Spot00_Break`** (80 lines, the broken drawbridge at the field's spawn 7, where the walk
   out ends). Recommended: in, with milestone 2. `En_Po_Field` and the rest of the adult field
   stay with Phase 13 (Epona).
6. **`En_Xc` and `Demo_6K` split by type,** the other types as placeholders chosen at init
   (Sheik's other songs with their phases 14 to 17, the reveal with 19). Recommended. Or: whole
   now (2,494 + 820).
7. **The coverage corrections** above, applied with milestone 1's coverage refresh. Recommended.
8. **BACKLOG to fold in:** #37 and #57 close with milestone 1, #26 with milestone 3. Re-file #53
   (Phase 17) and #55 (Phase 19). Recommended.

## Milestone 1: Adult Link and the time machine

**Status:** done: built and hand-tested 2026-10-23 (the blue ring round the pedestal fixed: "Found
by hand"), committed and pushed on the user's ask.

**The answer:** Link's age now changes in play, both ways, as the game changes it. The thirteen
cutscene destinations that only logged `linkAgeOnLoad` set it; `Play_Destroy` swaps the equips
(`Inventory_SwapAgeEquipment`, whole) while the old age is current, and `Player_Destroy` makes the
new age the save's. `Bg_Toki_Swd` (the Master Sword in its pedestal: the reveal, the pull, the
putting back, the temple's dimming) and `Door_Toki` (the closed Door of Time's collision) are
whole. Player has the adult's pedestal, `Player_PutSwordInHand`, start mode 1 with
`Player_Action_TimeTravelEnd`, and cutscene modes 18 and 30's updates. The adult's draw has its
gauntlet plates, iron and hover boots, the tunic's colour at draw time and the left hand's
overrides (the pedestal's, the broken Giant's Knife). ADR 0088.

Milestone 2's actors were ported alongside by a helper and merged: `En_Rl` (Rauru) whole, `Demo_6K`
types 14 to 19, `En_Xc` types 9 and 1 and the pedestal block (the other types placeholders chosen at
init). This closes BACKLOG #37 (room 0's placeholders) with #57 (the adult's pedestal).

Two scripted runs on debug presets:
- **The pull** (`Route::MasterSwordPull`, `master-sword`: entered by the hall,
  `ENTR_TEMPLE_OF_TIME_0`, then a child on the pedestal's step in room 0): to the sword, A (4:
  `gPullMasterSwordCs`, the Master Sword given); the child takes it on his frame 87 (his left hand
  holding it, the sword gone from the pedestal); the temple dims to 0xFF on cutscene flag 1;
  terminator 8 swaps the equips and he arrives an adult at `ENTR_TEMPLE_OF_TIME_2` on the
  pedestal, the Master Sword in hand, and stands (366). The end is the preset
  `master-sword-adult`.
- **The way back** (`Route::MasterSwordBack`, `master-sword-adult`: the adult arriving on the
  pedestal): standing (100), A (112: `gPlaceMasterSwordCs`); he lets go on his frame 70 (the sword
  drawn in the pedestal, his left hand closed); terminator 8 and he arrives a child with the Kokiri
  Sword and his own C buttons, the adult's kept (425).

Scripts: `scripts\run\game-master-sword.bat` (menu 169: empty, `back`, `door`),
`test-master-sword.bat` (170), `sandbox-master-sword.bat` (171).

### What was built

- **The age switch** (`oot_game`: `PlayState::link_age_on_load`, `SaveContext::inventory_swap_age_equipment`,
  `reinit`'s swap, the terminators; `Player::destroy`). Tests:
  `the_age_swap_gives_the_first_adult_his_equips_and_keeps_both_ages`,
  `the_first_adult_gets_deku_nuts_on_c_left_if_he_has_them`,
  `a_bottle_on_a_button_shows_its_slot_and_a_lost_shield_comes_off`.
- **`Bg_Toki_Swd`** whole (`bg_toki_swd.rs`; its bake `Bg_Toki_Swd/sword`), with
  `PlayerIface::set_interact_range_actor` for its write to Player. **`Door_Toki`** whole
  (`door_toki.rs`). Tests: `door_toki_blocks_the_doorway_until_the_door_is_open`,
  `the_pulls_time_warp_is_hidden_before_the_pull_unless_entered_from_the_chamber`.
- **Player's time travel:** the adult's branch of `Player_ActionHandler_2`, `put_sword_in_hand`,
  start mode 1 and `Action::TimeTravelEnd`, `func_80851A50` (mode 18), `func_80851D80` (mode 30),
  `Player::left_hand_dlists`. Tests: `the_time_travel_arrival_waits_twenty_frames_then_plays_its_animation`,
  `cutscene_mode_30_draws_the_sword_on_frame_6`, and the two runs.
- **The adult's draw:** `Loadout::left_hand`, `knife_broken`, `PlayerRules::hand_types`; Link's
  tunic colour from a dynamic segment (`SEG_TUNIC`); bakes `Player/gauntlets_*` (four, by the
  hands) and `Player/boots_iron`, `_hover`; the Link variants 150 to 220. **Pack format 33**,
  re-imported into `out/data27`. Tests: `the_left_hand_lists_for_the_pedestal_and_the_broken_knife`,
  `the_adults_gauntlets_boots_and_tunic_are_drawn`.
- **Milestone 2's actors** (the helper, merged): `en_rl.rs`, `demo_6k.rs`, `en_xc.rs` with their
  tests (`en_rl` 2, `demo_6k` 3, `en_xc` 7), bakes `En_Rl/opa`, `En_Rl/xlu`, `En_Xc/skel`, Rauru and
  Sheik in `ootx`'s characters.
- **The runs:** `Task::Pedestal`; the presets `master-sword`, `master-sword-adult`.
- **At the start:** the stale docs fixed (BACKLOG #37's `Bg_Spot02_Objects` 4, #53 re-filed to
  Phase 17, #55 to Phase 19; GAME-10's "What's left"; `demo_kankyo.rs`, `obj_timeblock.rs`), and
  the coverage corrections (below).

### Results

- Tests: `master_sword` (11), the helper's 12. 1,794 tests pass, 1 ignored; `cargo test -p layering`
  passes; no warnings.
- Goldens: all 337 of `out/golden_base40`'s hashes the same (the tunic's dynamic segment draws the
  same colour); 4 new cases (`master_sword_pull`, `_pull_cs`, `master_sword_back`, `_back_adult`): 343
  hashes, 255 cases, the same over two runs (`out/golden_base41`).
- COVERAGE (with the survey's corrections: `Bg_Toki_Swd`, `Demo_6K`, `En_Xc` and `Bg_Spot00_Break`
  to Phase 12, `En_Ru2` 16 and `En_Nb` 18, `kenjyanoma` 14/14/12 with layers 5 and 6 to 18 and 19,
  the Kokiri Forest adult layers to 14, the market's, its entrance's and the Temple's outside's
  adult layers never loaded): Phase 8 45 of 45, Phase 12 4 of 5 (`Bg_Spot00_Break` left, milestone
  2); 226 ported overall.

### Corrections to the survey

- **The first adult's C buttons aren't empty:** `Inventory_SwapAgeEquipment` puts Deku Nuts on
  C-Left if he has them, bombs on C-Down (owned or not) and the ocarina's slot on C-Right. From
  `ocarina-of-time`: nothing on C-Left, bombs (15) on C-Down, the Ocarina of Time on C-Right.
- `Item_Give(ITEM_SWORD_MASTER)` puts the Master Sword on the child's B as he pulls it (so his
  sheath is drawn without a sword through the cutscene); the swap stores the child's B as the
  Kokiri Sword whatever it holds.
- The adult's cry on `TimeTravelEnd`'s frame 158 never plays: his `link_demo_return_to_past` is 89
  frames long (the C's too).
- With the door already open `Demo_Kankyo` spawns no `Door_Toki`; and `Door_Toki` (flags 0)
  updates only inside its culling volume, so its collision turns off as Link nears it.
- A child pulling the sword a second time has always seen the Temple's intro (`EVENTCHKINF_A7`),
  come back from the Chamber (0xC9), seen Sheik's awakening (0xC5) and learned the Prelude (0x55:
  he became a child again only by putting the sword back, which needs it). So `master-sword` has
  them all. Without 0xC5 the arriving adult gets Sheik's awakening; without 0x55 Sheik
  (`En_Xc` 9, placed at (0, 71, 11)) stands by the pedestal and her body pushes the arriving adult
  27 back.

### Found by hand (2026-10-23)

- **A blue ring round the pedestal before the pull** (the user, `game-master-sword.bat`'s first
  start): the pull's time warp (`Demo_Effect` 0xFF0F). `DemoEffect_DrawTimeWarp` draws it on
  cutscene flag 1, on a cutscene layer, or whenever the entrance is `ENTR_TEMPLE_OF_TIME_4` (the
  adult's return from the Chamber; with 0xC9 set it stays on the pull's first frame): the C's, and
  the debug start had entered by `ENTR_TEMPLE_OF_TIME_4`, which a child never takes. The start now
  enters by the hall and moves to room 0 (`--room 0 --at 0,70,46,-3641`); test
  `the_pulls_time_warp_is_hidden_before_the_pull_unless_entered_from_the_chamber`. The pull's two
  goldens re-recorded (their start).

### Things to check, answered

- **The closed Door of Time:** before this milestone a child could walk through it (`Door_Toki`
  was a placeholder); now its collision blocks the doorway until `EVENTCHKINF_OPENED_DOOR_OF_TIME`.
- **The pull's dimming:** `SDC_TEMPLE_OF_TIME` reads `roomCtx.drawParams[0]`, which `Bg_Toki_Swd`
  steps to 0xFF (the runs check it).
- **The first adult's equips:** the C's (above); the equipment page's stand-in has nothing to do
  for him.

### Known gaps

- Not seen in a window yet (the hand test); the headless shots look right (the child holding the
  sword, the adult's arrival with the HUD's new buttons, the child's return).
- From the helper: Rauru's translucent fade (`En_Rl/xlu`, `gActorSetupXluDL` standing in for
  `Gfx_SetupDL_25Xlu`'s render mode) isn't confirmed by eye; `func_80AE7668` turns Player through
  `actors.actor_mut` (as `Boss_Dodongo` does) rather than `PlayerIface`; Rauru's and Sheik's arm
  limbs in `ootx`'s characters are left empty; `func_80AE73D8`'s static flag is per instance.
  Milestone 2 runs these actors in their cutscenes.
- The equipment page stays the stand-in (Phase 20, the user's call 3).

### How to check

```
scripts\run\test-master-sword.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-master-sword.bat
```

### By hand

`scripts\run\game-master-sword.bat` (menu 169). WASD the stick, Space A, E B, J/K/L the C buttons,
Enter Start.

1. **Empty** (a child on the pedestal's step behind the Door of Time): turn to the sword ("Grab" on
   A) and press A. The pull: Link climbs up, takes the hilt in both hands and draws it (his left
   hand holding the sword, the sword gone from the pedestal), the temple darkens, the time warp's
   blue, the white fill. Then the adult on the pedestal with the Master Sword in hand: after a
   moment his arrival plays (the sword's swing sound) and he stands. Check: the HUD's B is the Master Sword, C-Left empty, C-Down
   bombs (15), C-Right the Ocarina of Time; the Hylian Shield on his arm, the green tunic, no
   gauntlets or boots drawn. Before the pull: no blue ring round the pedestal (the time warp shows
   only once the cutscene's flag 1 is set).
2. **`back`** (the adult arriving on the pedestal, the Prelude's flag set): wait for him to stand,
   then press A facing the sword's place ("Drop"). He puts the sword back in (it stays drawn in
   the pedestal, his left hand closed), the white fill, and he arrives a child who jumps off the
   pedestal. Check: B the Kokiri Sword, C-Left sticks, C-Down the boomerang, C-Right the slingshot;
   the sword in the pedestal. Then press A at it again: he pulls it and is an adult again (the
   same equips as in 1).
3. **`door`** (the Temple's entrance with the door shut, `ocarina-c`: its intro plays first): walk
   to the Door of Time behind the altar: it stops Link (before this milestone he walked through
   it). Take the ocarina out at the altar (C-Down) and play the Song of Time: the door opens and
   lets him through.

## Milestone 2: The Door of Time to the Chamber of the Sages and out

**Status:** done: built and hand-tested 2026-10-23 ("it works"), committed and pushed on the
user's ask.

**The answer:** the story now plays from `ocarina-of-time` to adult Link in Hyrule Field, in five
short runs chained by presets:
- the Temple's intro, the Ocarina of Time put on C-Down from the item page, the Song of Time at the
  altar and the door opening (`door-of-time`);
- the reveal, the first pull, Ganondorf on the cutscene map, the Chamber of the Sages with Rauru and
  the Light Medallion (`light-medallion`, the adult's first save state);
- Sheik's awakening (`adult-awake`, **Phase 13's start**);
- out of the Temple to `shrine_r`, **the exit**;
- on through the ruined market (round its eight ReDeads, no fight), its entrance and the field by
  the broken drawbridge.

Ported: `SDC_CHAMBER_OF_THE_SAGES`, `chamberCutsceneNum` (destinations 29 to 31, outside the save)
and `Bg_Spot00_Break` whole, which completes Phase 12's coverage row. Milestone 3's `En_Xc` type 2
(the Prelude) was ported alongside by a helper and merged (pack format 34). ADR 0089.

Scripts: `scripts\run\game-temple-of-time.bat` (menu 172: empty, `pull`, `chamber`, `awakening`,
`temple`, `outside`, `market`, `field`), `test-temple-of-time.bat` (173),
`sandbox-temple-of-time.bat` (174).

### What was built

- **`SDC_CHAMBER_OF_THE_SAGES`** (`scene_table.rs`, `z_scene_table.c:408`): the light's scroll on
  segment 8, the floor's two on 0xA, the falling light's on 9; in `oot_import`'s pack test against
  the C interpreter (`kenjyanoma_scene`).
- **`chamberCutsceneNum`** (`SaveContext::chamber_cutscene_num`, `CHAMBER_CS_*`):
  - `SaveContext_Init` and the file select start it at `CHAMBER_CS_FOREST`; destinations 29 to 31
    set it; the slot's copy of `SaveContext` holds it at 0x1415.
  - Its readers (`Demo_Sa` 0 and 1, `Demo_Du` 0, `Demo_Im` 0 and 1) are later phases'
    placeholders: each logs when the number would start its medallion's script (BACKLOG #110).
    `Door_Warp1`'s adult warps, the other writer, wait for Phase 14.
- **`Bg_Spot00_Break`** whole (`bg_spot00_break.rs`): the broken drawbridge (params 0) and three
  barbed wire fences (1) on the field's adult layers 2 and 3, each with its collision and display
  list; killed for a child. Tests `the_adults_field_has_the_broken_drawbridge_and_the_fence`,
  `a_childs_is_killed_at_init_and_its_collision_goes`.
- **The runs** (`playthrough.rs`; tests in `temple_of_time_runs.rs`): `Route::DoorOfTime`,
  `FirstPull`, `Awakening`, `TempleExit`, `Ruins`; the presets `door-of-time`, `light-medallion`,
  `adult-awake`. Two task fixes:
  - `Task::WalkUntil` now takes its points in turn (it turned Link back and forth at a waypoint).
  - `Task::MenuCursor` takes the cursor's own moves (`KaleidoScope_DrawItemSelect`'s: empty slots
    skipped, rows wrapped) where its row-then-column way would loop, as it did from the bombs to
    the ocarina's slot.
  Earlier runs' inputs are unchanged (all 343 old hashes the same).
- **The helper's open points:** Rauru's and Sheik's arms filled in `ootx`'s characters (their
  limbs are unnamed: the chest's two three-limb branches, Ruto's pattern); `En_Rl`'s docs on its
  static (per instance, one Rauru per scene) and on Link's turn (below).
- **`En_Xc` type 2** (the helper, merged):
  - `EnXc_Init`'s type 2 (through the beta Serenade's pass-through to action 30);
  - actions 29 to 44, and the shared actions 6 to 18 they call;
  - the harp's draws (`EnXc_DrawPullingOutHarp`, `EnXc_DrawHarp`; bake `En_Xc/harp`);
  - six tests.
  Pack format 34, re-imported into `out/data27`. Milestone 3 runs it in `gTempleOfTimePreludeCs`.

### Results

- The five runs (frames):
  - the door: 1,392 (the song played on 439, the door's flag inside the script's frames 510 to 790);
  - the first pull: 5,599 (the reveal 361, the pull 1,050, Ganondorf 1,281, the Chamber 2,016, the
    medallion 5,067);
  - the awakening: 2,389 (`EVENTCHKINF_C9` on the time warp's 251st update);
  - the way out: 548;
  - the ruins: 1,015 (the nearest ReDead 170 away).
- Each run ends as its preset. The texts are the C's, in order: 0x70E6 to 0x70E8, 0x700A, 0x704E,
  0x704F, 0x7050, 0x7051, 0x7053, 0x0040, 0x70E9; then 0x7054, 0x7056, 0x7057, 0x7077 to 0x707A.
- Tests: `temple_of_time_runs` (6), `bg_spot00_break` (2), `en_xc` (12, 6 new), the
  `SDC_CHAMBER_OF_THE_SAGES` row in `oot_import`'s pack test. 1,807 tests pass, 1 ignored;
  `cargo test -p layering` passes; no warnings.
- Goldens: all 343 of `out/golden_base41`'s hashes the same; 11 new cases (each run's trace and end,
  the stones, Ganondorf, the Chamber, Sheik, the ruined market, Rauru's fade on layer 6): 359
  hashes, 266 cases, the same over two runs (`out/golden_base42`).
- COVERAGE: Phase 12 5 of 5; 227 ported overall.

### Things to check, answered

- **Who loads the Temple's layers 4 and 6:** no one in play. No destination enters the Temple with
  `CS_INDEX_0` or `CS_INDEX_2`; `En_Xc` 9 plays their scripts (`gTempleOfTimeFirstAdultCs`,
  `gTempleOfTimePreludeCs`) itself on the adult's layer 2. The layers are the map select's.
- **Adult Link in the ruins' prerendered scenes:** `shrine_r` (`PREREND_FIXED`, bg cameras 0 and
  1), `market_ruins` (`SCENE_CAM_TYPE_FIXED_MARKET`: its one bg camera, `PREREND_PIVOT` with the
  room's skybox) and `enrui` (`PREREND_FIXED`) all take him; the runs walk through them.
- **The ReDeads' range:** `EnRd_Idle` notices Link only when he makes a noise (`PLAYER_STATE2_3`: a
  footstep faster than 4, a swing) within 150 across. The square's middle is a round fountain (its
  wall 107 from (0, 0)). Round its west side every ReDead stays over 160 away, so `Route::Ruins`
  goes round them.
- **`ENTR_TEMPLE_OF_TIME_4`'s spawn** (0, 70, 46), facing -0x0E39: Sheik's script moves Link at
  once (its first player cue at (0, 70, 30), facing -0x8000). It leaves him at (0, 40, 214) facing
  30037 with the Master Sword in hand (mode 0x30's `func_80851D80`). The way out starts there.
- **Rauru's translucent fade** (`En_Rl/xlu`): looked at on the Chamber's layer 6 (type 2, cue 4):
  half see-through on frame 25, solid by 45 (golden `chamber_rauru_fade`). Type 0, in the Light
  Medallion's script, is opaque throughout, as the C draws it.
- **`func_80AE7668`'s turn of Link:** it lands (Player is in the arena during Rauru's update) and
  stays an actor write. The C writes only `world.rot.y` and `shape.rot.y`; `PlayerIface::set_yaw`
  would also write `player->yaw`. In the script it gives the yaw the first player cue gives anyway
  (Rauru faces 0x4000, Link -0x4000).

### Corrections and findings

- The altar's song spot takes the ocarina only within 50 (Phase 11's lesson again): spawn 3's
  place, 66 away, plays the song freely. The run plays it 31 away.
- The hall's altar is 41 over the floor in front of it; the way to the doorway is round its sides,
  whose floor rises in 10-high steps.
- The first pull's run ends as terminator 18's transition starts: on the Temple's next `Play_Init`,
  `En_Xc` 9 sets `EVENTCHKINF_C5` at once, so `light-medallion` is the save before it.

### Known gaps

- `chamberCutsceneNum`'s medallion scripts and `Door_Warp1`'s adult warps wait for their phases
  (BACKLOG #110).
- `En_Xc` type 2 isn't run in its cutscene yet (milestone 3). The helper left `EnXc_InitFlame`'s
  Bolero flame (Phase 15) and type 8's boots check (a placeholder type) logged.

### How to check

```
scripts\run\test-temple-of-time.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-temple-of-time.bat
```

### By hand

`scripts\run\game-temple-of-time.bat` (menu 172). WASD the stick (Shift walks), Space A, E B,
J/K/L the C buttons, Enter Start.

1. **Empty** (the Temple's entrance, a child, the door shut): the intro plays (the camera down the
   hall). Enter for the item page; move to the Ocarina of Time (second row, second slot) and press
   K to put it on C-Down; Enter again. Walk up to the altar and stop just in front of it, press K,
   and play the Song of Time: L, Space, K, L, Space, K. Check: the stones fly to the altar, the
   door slides open, the light over it goes; Link is free after.
2. **`pull`** (the door open): walk round the altar's side (its middle is a step too high) and
   through the doorway into the pedestal's room: Navi flies to the sword ("...isn't that...?!",
   "The Master Sword!!"). Walk to the sword and press A: the pull, the white-out, Ganondorf's laugh
   on the cutscene map, then the Chamber: Rauru's texts (take the first answer to his question),
   the Light Medallion over Link, "Find the other Sages", the black fade. Then the Temple as an
   adult: Navi's and Sheik's texts, the time warp shrinking round the pedestal, Link drawing his
   sword. Check: the HUD's B is the Master Sword, C-Down bombs, C-Right the ocarina; Link is an
   adult throughout the Chamber.
3. **`chamber`**: the Chamber's script alone (an adult): the floor's light scrolling, Rauru.
4. **`awakening`**: the adult arriving at the pedestal's step: Sheik's whole scene.
5. **`temple`**: the adult free where the awakening leaves him. Sheik stands in front of the
   pedestal (A at her: "...another skill..."). Walk out through the doorway and down the hall.
6. **`outside`**: the Temple's steps in the ruins: the fixed camera and its background, the gossip
   stones, Death Mountain's smoke ring. Walk to the market.
7. **`market`**: the ruined market. Walk round the fountain to the south exit. Then try a ReDead:
   run up close (it screams, Link freezes, it grabs him: mash the stick and A to break free); walk
   slowly near another (it doesn't hear him).
8. **`field`**: Hyrule Field by the market's broken drawbridge (behind Link): its rubble is solid
   to stand on; the barbed wire fences by the Lost Woods' bridge and the river.

## Milestone 3: Sheik's Prelude, the sword put back, the warp

**Status:** done: built and hand-tested 2026-10-24 ("that works"), committed and pushed on the
user's ask.

**The answer:** time travel now runs both ways in the story's state, and the Prelude of Light is
taught and warps:
- Sheik's Prelude of Light: the adult, with the Forest Temple's flag, walks into the pedestal's
  room; `En_Xc` 9 gives the song and plays `gTempleOfTimePreludeCs`: her harp, the song played
  back, her Deku Nut and she's gone (`prelude-learned`);
- the Master Sword put back: a child again with his own buttons (`child-again`);
- pulled again: an adult again;
- the Prelude played in the field: "warp?" yes, the warp-out with Link gone in its white-out, the
  arrival on the Temple's light pad, hidden until the warp-in's script gives him back.

The Prelude needs `EVENTCHKINF_48`, the Forest Temple's (Phase 14), so its run starts from a debug
preset, `prelude` (the user's call 2). BACKLOG #26 closes: Player's draw switch. ADR 0090.

Scripts: `scripts\run\game-prelude.bat` (menu 175: empty, `back`, `again`, `warp`),
`test-prelude.bat` (176), `sandbox-prelude.bat` (177).

### What was built

- **Player's draw switch** (`actor.draw` NULL): `ActorImpl::has_draw`, which `Actor_DrawAll`
  tests after the culling as the C does (an actor without a draw isn't drawn, nor its shadow).
  Player's `hidden` is set by `Demo_Kankyo`'s warp-out on its timer's 15
  (`PlayerIface::set_draw`) and by start mode 5, and cleared by `func_80852280` (the warp-in
  script's first action, through `Player_Action_WaitForCutscene`, or cutscene action 46). The sub-timer
  half of #26 came with the timers' port earlier.
- **The runs** (`playthrough.rs`; tests in `time_travel_runs.rs`): `Route::Prelude`,
  `SwordBack`, `SwordAgain`, `PreludeWarp`; the presets `prelude` (a debug start), `prelude-learned`,
  `child-again`.
- **`En_Xc` type 2** (milestone 2's helper, merged then) runs whole in its script here.

### Results

- The four runs (frames):
  - the Prelude: 2,339 (its script from 357, the song played back 1,559);
  - the sword put back: 456 (the cutscene from 143), a child at `ENTR_TEMPLE_OF_TIME_2`;
  - pulled again: 446 (from 84), an adult;
  - the warp: 414 (the song heard 77, "warp?" answered 215), at `ENTR_TEMPLE_OF_TIME_7`.
- The texts are the C's: 0x7074, 0x708B, 0x708C, the playback's 0x0875 and 0x086D, 0x0078 ("You have
  learned the Prelude of Light!"), 0x7075; the warp's 0x086E, 0x086F, 0x0898 and "warp?" 0x0892.
- Sheik draws plain, then pulling out the harp, then the harp, and back; she throws her nut (action
  43) and is gone on cue 9.
- Link is hidden from the warp-out's timer's 15 to the scene change, and after the arrival until the
  warp-in's script takes him (40 frames and more), drawn after.
- Tests: `time_travel_runs` (4). 1,811 tests pass, 1 ignored; `cargo test -p layering` passes; no
  warnings.
- Goldens: all 359 of `out/golden_base42`'s hashes the same (no earlier case plays a warp song); 6 new
  cases (each run's trace and end, Sheik, the warp's white-out): 369 hashes, 272 cases, the same
  over two runs (`out/golden_base43`).

### Corrections and findings

- **The swap keeps what it stored:** pulling again leaves `adultEquips` holding the adult's buttons
  (the put-back stored them; nothing clears them), and `childEquips`' equipment the Master Sword's
  nibble (the pull's `Item_Give` puts it on the child before the swap stores his equipment as it
  is). Both are the C's (`Inventory_SwapAgeEquipment`); the pulled-again run is checked against
  `child-again` with the pull and the swap applied, not against `prelude-learned`.
- `En_Arrow`'s init makes Sheik's `ARROW_CS_NUT` an `ARROW_NUT` that also updates in cutscenes.

### Known gaps

- The Prelude's run starts from a debug preset until Phase 14 brings the Forest Temple.

### How to check

```
scripts\run\test-prelude.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-prelude.bat
```

### By hand

`scripts\run\game-prelude.bat` (menu 175). WASD the stick (Shift walks), Space A, E B, J/K/L the C
buttons, I C-Up, Enter Start.

1. **Empty** (the Temple's entrance, an adult with the Forest Medallion's flag): walk round the
   altar's side and through the doorway into the pedestal's room: Sheik's scene. She pulls out her
   harp and plays; play the song back when the staff shows: I, L, I, L, J, I (C-Up, C-Right, C-Up,
   C-Right, C-Left, C-Up). "You have learned the Prelude of Light!", her last words, she backs off,
   throws a Deku Nut and is gone in its flash.
2. **`back`** (the adult in the pedestal's room, the Prelude learned): face the pedestal, A ("Drop"):
   the sword goes back in, the white-out, a child jumping off the pedestal. Check: B the Kokiri
   Sword, C-Down the Ocarina of Time.
3. **`again`** (that child arriving on the pedestal): A at the sword: pulled again, an adult with the
   Master Sword on B, bombs and the ocarina.
4. **`warp`** (Hyrule Field by the broken drawbridge, an adult): L for the ocarina, play the
   Prelude (I, L, I, L, J, I), "Yes": the warp's sparkles and white-out (Link should vanish into
   the light, not stay standing in it), then the Temple's light pad: Link appears with the sparkles
   after a moment and is free.
