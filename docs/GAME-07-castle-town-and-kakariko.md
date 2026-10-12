# GAME-07: Castle Town, Hyrule Castle and Kakariko

**Goal:** Phase 8 of the [roadmap](ROADMAP.md): child Link's first pass through the world beyond
Hyrule Field. Castle Town by day and by night, Hyrule Castle (Malon, Talon and the crates, the
guards, Zelda's courtyard, her letter, Impa's escort and Zelda's Lullaby), and Kakariko with its
graveyard, by day and by night. The ocarina comes with it: Zelda's Lullaby is the first song
taught.

**Status:** decided (2026-10-11). The user chose:
- the six milestones below, in the story's order; the shops and games left out;
- every actor whole, except `En_Ta` (the castle's and Kakariko's parts), `Demo_Im` (types 3, 4
  and 5), the actors killed for a child (`En_Xc`, `Demo_6K`, `Bg_Toki_Swd`, `En_Go2`: their inits'
  checks), and the ocarina's scarecrow, memory game and frog parts;
- the ocarina as its own milestone, before Zelda's;
- the culling volume (BACKLOG #32) at the start of milestone 1; `En_Bom` (#33) later.

| # | Milestone | Status |
|---|---|---|
| 1 | Castle Town: the culling volume, the entrance, the market, the back alley and the Temple of Time's outside by day and by night, the Temple's altar, the townsfolk, the dogs, Malon | done (2026-10-11) |
| 2 | Hyrule Castle's grounds: the Weird Egg and its hatch, Player's exchange items, Talon, the crates, the guards and the throw-out, the circle wipe | |
| 3 | The ocarina: Player's, the message modes, the audio, the songs' effects, `Shot_Sun`, the Sun's Song | |
| 4 | The guards' courtyard and Zelda: `Camera_Special6`, the patrols, Zelda, the legend, the letter, Impa's lullaby and escort | |
| 5 | Kakariko: lifting and the cuccos, Anju, the gate, the people, the interiors | |
| 6 | The graveyard: Dampé, the royal tomb, the ReDeads, the Sun's Song | |

The working rules are the same as for the earlier phases:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack, and only the importer reads the decomp;
- ports go function by function with the decomp's names, every constant is cited, and faithful
  bugs are marked `@bug (game)`;
- test expectations come from the C;
- a change in a run's trace is a golden change to explain.

The decomp is zeldaret/oot main at `52a510f` ([ADR 0031](adr/0031-decomp-main.md)). Decisions
are in [docs/adr/](adr/README.md) (0058 on).

## The survey

Line counts are the decomp's (`52a510f`), whole `.c` files of each overlay unless said otherwise.
"Ported" means `oot_actors::overlays()` registers the actor today (`225e88a`, the end of GAME-06).
Each scene layer's actors are the pack's (`ootx scene-info --scene <file> --layer N`); the
scripts are the pack's (`ootx cutscene <symbol>`).

**The layers.** Layer 0 is the child by day, 1 the child by night, 2 and 3 the adult's, 4 on the
cutscenes'. Most interiors have no alternate headers: `Scene_CommandAlternateHeaderList` then
keeps layer 0's commands, so `scene-info` shows layer 0 again for layers 1 to 3. Castle Town's
day and night are separate scenes (`entra` and `entra_n`, `market_day` and `market_night`,
`market_alley` and `market_alley_n`, `shrine` and `shrine_n`): the entrance table picks the
scene by the layer. The port already does this: `Route::Field` arrives at `entra` by day.

**The scene files' names mislead:** `labo` is Impa's house, `impa` is the dog lady's house (in
Castle Town's back alley), `kakariko` is Kakariko's guest house, `kakariko3` is the back alley's
house, and `hakasitarelay` is the windmill (room 6) with Dampé's grave.

### Sizes

Unported actor kinds on the child's layers (0 and 1), each kind counted once, whole files:

| Area | Scenes | Kinds, placements (day + night) | Unported kinds | Unported lines |
|---|---|---|---|---|
| Castle Town, the story's scenes | the entrance, the market, the back alley, the Temple of Time's outside (each by day and by night), the Temple of Time | 27, 258 | 17 | 9,911 |
| Castle Town's shops and games | the potion shop, the Bombchu shop, the mask shop, the dog lady's house, the treasure chest game, Bombchu Bowling, the guard house | (with the above) 40, 462 | +9 | +2,623 (12,534 in all) |
| Hyrule Castle | the castle grounds, the guards' courtyard (day, night), Zelda's courtyard | 27, 164 | 13 | 8,657 |
| Kakariko, the village and the graveyard | `spot01`, `spot02` | 46, 275 | 25 | 10,008 |
| Kakariko's interiors | the guest house, the back alley house, the bazaar, the potion shop, Impa's house, the House of Skulltula, the windmill, Dampé's hut, Granny's shop, the graves, the royal tomb, the shooting gallery | (with the above) 69, 573 | +15 | +10,213 (20,221 in all) |

Some of those lines never run for a child: `En_Xc` (Sheik, 2,494), `Demo_6K` (820),
`Bg_Toki_Swd` (465) and `En_Go2` (the bazaar's Goron, 2,113) kill themselves at their init for
a child without the stones, and `En_Ta`'s Lon Lon Ranch half (about 670 of 1,344) waits for the
ranch. Without them, the three areas' story and their people come to about 22,000 lines of
actors, plus the systems below (about 6,500).

### 1. Castle Town

**The prerendered scenes (ADR 0014).** Castle Town has three kinds of view:
- **Fixed pictures** (`CAM_SET_PREREND_FIXED`, `SCENE_CAM_TYPE_FIXED`): the drawbridge's entrance
  (one bg camera), the back alley (three), the Temple of Time's outside (two). Each is a JPEG
  background drawn as a screen quad, as Link's house's fixed view is today. BACKLOG #1 (the room's
  own geometry beyond the picture at 16:9) applies to all of them.
- **The market's square** (`SCENE_CAM_TYPE_FIXED_MARKET`, `CAM_SET_PREREND_PIVOT`): a pivot view
  inside a 360° room skybox, `SKYBOX_MARKET_CHILD_DAY` / `_NIGHT` (`SKYBOX_DRAW_256_4FACE`), as the
  houses' pivot view is today (`Camera_Unique7`, the skybox bakes). Its second bg camera,
  `CAM_SET_MARKET_BALCONY`, is `Camera_Fixed1` (about 70 lines): not ported.
- **The Temple of Time** inside: a normal 3D scene (`CAM_SET_START0`).

`SCENE_CAM_TYPE_FIXED_MARKET` also moves the message box (`z_message.c` 4177) and is checked by
`Play_CamIsNotFixed` and the camera bgcheck (`z_bgcheck.c` 1574).

**The people.** By day, the market has 13 townsfolk, Malon, the dog lady's dog, the haggling
couple, the girl chasing her cucco, the dancing couple; by night, 7 dogs, two guards and one
townsman. Every scene's guard tells Link to go home at night (`En_Heishi4` type 8).

| Actor | Lines | Ported | Where | What |
|---|---|---|---|---|
| `En_Hy` | 1,524 | no | the market (13 by day, 1 by night), the back alley (3), the shops, Kakariko's interiors | The townsfolk: 21 types (`params & 0x7F`), each its own objects (head and body from different objects), path (`params >> 7`), texts and spawn rules (`EnHy_ShouldSpawn`). Behaviours: walk a path, pace, watch the dog, fidget. The beggar buys bottled fish, bugs and blue fire; the dog lady gives a piece of heart for Richard |
| `En_Heishi4` | 407 | no | the entrance, the back alley, the market by night, the guard house; Kakariko (2) | Guards that talk (`{TIME}` in Kakariko's text), track Link with their heads. No catching |
| `En_Dog` | 498 | no | the market (1 by day, 7 by night), the back alley by night (3), the Temple of Time's outside by night (2) | Dogs. At night one touched follows Link (`gSaveContext.dogParams`), across scenes: Player respawns it behind him (`z_player.c` 12109). Richard is there only while `dogIsLost` (reset each dawn) |
| `En_Ma1` | 490 | no | the market (by day and by night), the castle | Malon, singing (`Audio_ToggleMalonSinging`). **In the market until the castle has been entered once:** the castle's first load sets `INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE` and doesn't spawn her (`EnMa1_ShouldSpawn`); from the next load she's at the castle and gone from the market. She gives the Weird Egg at the castle only |
| `En_Mu` | 234 | no | the market (2) | The haggling couple: rotating texts |
| `En_Niw_Girl` | 283 | no | the market | The girl chasing her cucco (an `En_Niw` of type 0xA she spawns: on a path, not carryable) |
| `En_Tg` | 199 | no | the market (and Kakariko as an adult) | The dancing couple |
| `En_Gs` | 633 | no | the Temple of Time's outside (4); the castle (2); the graveyard | Gossip stones: their texts (the hints with the Mask of Truth), a fairy for any song, a big fairy for the Song of Storms |
| `Obj_Mure` | 442 | no | the market; the castle (4); Kakariko; the graveyard | Groups of fish, bugs or butterflies (`En_Butte` 453, `En_Insect` 816, `En_Fish` 723: here only butterflies). A butterfly turns into a fairy for a Deku Stick held |
| `En_Kusa`, `En_Wood02`, `Obj_Kibako2`, `En_Wonder_Item`, `En_River_Sound`, `En_Door`, `Bg_Spot16_Doughnut` | | yes | | |

**The Temple of Time** (`tokinoma`). A child without the stones reaches room 1 only: the altar.
`gTempleOfTimeIntroCs` plays on the first entry (`EVENTCHKINF_A7`). There:
`En_Wonder_Talk` (266: the altar's text, also the graveyard's inscriptions), `Bg_Toki_Hikari`
(185: the light through the windows), `Bg_Mjin` (135: the warp pad), `En_Okarina_Tag` type 4
(the Door of Time, the Song of Time), `Demo_Effect`'s jewels and `Demo_Kankyo` (ported).
Behind the door, room 0: `Bg_Toki_Swd`, `Demo_6K`, `En_Xc` (killed for a child),
`Bg_Spot02_Objects` type 4.

**The shops and games**, none on the story's path:

| Place | Actors | Lines | For a child |
|---|---|---|---|
| The potion shop (`alley_shop`) | `En_Ossan` type 3, `En_Hy` type 19, `En_Tana` | per type, about 100 of `En_Ossan`'s 2,548 | open |
| The bazaar (`shop1`, shared with Kakariko) | `En_Ossan` type 4, `En_Hy` type 20, `En_Go2` type 0xD (killed without the Goron's Ruby) | | open |
| The mask shop (`face_shop`) | `En_Ossan` type 10 (killed until `INFTABLE_76`: the Kakariko guard has the letter), `En_Guest` (254) | | after the letter |
| The Bombchu shop (`night_shop`) | `En_Ossan` type 2 (killed until `EVENTCHKINF_25`: Dodongo's Cavern) | | no |
| The treasure chest game (`takaraya`, by night) | `En_Takara_Man` (245), `En_Changer` (310), `Door_Shutter` (ported, small key doors), `En_Box` (ported), `Item_Etcetera` (261) | 816 | yes |
| The shooting gallery (`syatekijyou`, by day) | `En_Syateki_Itm` (359), `En_Syateki_Man` (536), `En_G_Switch` (594), `En_Ex_Ruppy` (411); Player's gallery mode (the slingshot forced, 15 shots) | 1,900 | yes |
| Bombchu Bowling (`bowling`) | `En_Bom_Bowl_Man` (515: "not open yet" until `EVENTCHKINF_25`), `Bg_Bowl_Wall`, `Bg_Bom_Guard`, `En_Trap` | 1,247 | closed |
| The guard house (`miharigoya`) | `En_Heishi4`, 44 `Obj_Tsubo` (pots), `En_Light` | | yes |
| The dog lady's house (`impa`) | `En_Hy` type 0 (by night) | | yes |

The shops reuse what GAME-03 ported: `En_Ossan`'s states are all ported, with the Kokiri
shopkeeper's init and draw only. Each other shopkeeper is its init, its objects, its draw and its
hello text; `En_GirlA` (ported) needs its other items' bakes.

### 2. Hyrule Castle

**The way in** (`spot15`, child only; the adult's is Ganon's Castle's outside). From the market's
north exit, `gHyruleCastleIntroCs` plays on the first entry (`EVENTCHKINF_A5`: a camera flight
with the place name, stopped at frame 125). Then:
1. **The gate guard** (`En_Heishi2` type 2, at the gate to Lon Lon's fence): "won't pass"
   (0x7029); with the Weird Egg, a bribe (10 rupees) that opens the fence (`Bg_Spot15_Saku`, 114),
   with a sub camera for up to 200 frames. The fence's opening doesn't persist (`INFTABLE_71` is
   set only by a legacy text handler). Neither this nor the fence is on the way to Zelda.
2. **Up the vines** (`WALL_TYPE_4`, x -652 to 92) to the upper level, past the castle grounds'
   guards (`En_Heishi3`, 271 lines, 9 placed): seven sentries see Link within 250 by day (200 by
   night) in front of them, 150 (100) behind; the two at the castle's door watch a box. Seen:
   "Hey you!" (0x702D, `NA_SE_SY_FOUND`), the guard walks over, and Link is thrown out to
   `ENTR_HYRULE_CASTLE_4` (by the gate) with a starburst circle wipe.
3. **Malon** (`En_Ma1`, from the castle's second load) gives the **Weird Egg**
   (`GI_WEIRD_EGG`, `EVENTCHKINF_RECEIVED_WEIRD_EGG`).
4. **The egg hatches at dawn:** `Environment_PlayTimeBasedSequence`'s `TIMESEQ_DAY_BEGIN_SFX`
   (`z_kankyo.c` 2218) counts the day (`totalDays`, `bgsDayCount`, `dogIsLost`), crows
   (`NA_SE_EV_CHICKEN_CRY_M`) and swaps the egg for the Cucco (`Inventory_ReplaceItem`, ported),
   with text 0x3066. `Play_Init` does the same when the Sun's Song moves to day
   (`NEXT_TIME_DAY`). The port's clock has the music's half of it (GAME-06 milestone 3); the
   counters and the swap are left.
5. **Talon** (`En_Ta`, castle part about 150 lines plus 46 shared with Kakariko) sleeps by the
   crates (0x702A). Shown the Cucco (`EXCH_ITEM_CHICKEN`), he wakes (0x702B, `NA_SE_VO_TA_SURPRISE`),
   talks, and on 0x702C's event runs off to the ranch on one-point cutscene 4175
   (`EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`).
6. **The crates** (`Bg_Spot15_Rrbox`, 381): pushed and pulled (Player's push and pull, ported in
   GAME-05 milestone 4c), 20 units a push; they fall where there's no floor, stack (the chime when
   one sits square on another), and push towards the moat only once Talon is gone. Two pushed into
   the moat stack under the ledge (770, 1490 / 1550, -299).
7. **Up the crates to the ledge (y 1617) and into the drain:** a crawlspace (`WALL_TYPE_5` at
   x 560; the crawl is ported) to exit 2, the guards' courtyard. The moat is a water box (y 1510);
   Link swims there (ported).

**The guards' courtyard** (`hairal_niwa`): a garden seen side-on, screen by screen. Its nine bg
cameras are `CAM_SET_PREREND_SIDE_SCROLL`, which is `Camera_Special6` (about 100 lines; not a
picture: the 3D scene with a camera that slides along each screen, freezing Link 12 frames at a
screen change). Eight patrolling guards by day (`En_Heishi1`, 526), two by the drain by night.
- **Their sight** is a line, not a cone: each frame each guard throws a search ball
  (`Effect_Ss_Solder_Srch_Ball`, 61 lines) 30 a frame along his head's direction, stopped by
  walls; it sees Link within 70. Near him (60, or 70 unless falling) he sees Link too. A guard
  only looks while the camera shows his screen (`sBgCamIndices`).
- **Caught:** "Hey you!" (0x702D), he runs to Link, and Link is kicked out to
  `ENTR_HYRULE_CASTLE_3` (by Talon's crates) with a starburst circle wipe.
- `@bug (game)`: type 5's animation entry reads past its table; the catch's `velocity.y > -3.9`
  check lets a sidehop through.

**Zelda's courtyard** (`nakaniwa`, `SDC_CALM_WATER`). Placed on layer 0: Zelda (`En_Zl4`, 1,527 with
her camera tables), Impa (`Demo_Im` type 5, invisible), two guards seen through the window
(`En_Heishi2` type 6: a static list), Ganondorf (`En_Viewer` type 8, ported), a Triforce spot
(`Demo_Effect`, inert here), `Demo_Kankyo`, two wonder items. Zelda's meeting is her own code
with the main camera (`CAM_SET_FREE0`, `CAM_SET_CS_0`), 14 fixed views and 11 moves:
1. She offers to talk from in front (the talk snaps Link 56 in front of her): `NA_BGM_APPEAR`,
   texts 0x702E, 0x702F; `gZeldasCourtyardMeetCs` runs for its Navi cues; then the stone (0x7030,
   a choice that repeats on "no"), the names (0x7033, the same).
2. At frame 130 of the names she sends Link to the cutscene map with `CS_INDEX_7`: layer 11,
   `gCutsceneMapTriforceSheikahLegendCs` (`Demo_Effect`, `Demo_Kankyo`: ported). Its terminator
   (`CS_DEST_ZELDAS_COURTYARD_RECEIVE_LETTER`) sets noon and enters `ENTR_CASTLE_COURTYARD_ZELDA_1`.
3. Back, from `EnZl4_SetupFromLegendCs`: the legend's questions; "look through the window"
   (`gZeldasCourtyardWindowCs`: Link walks to it; `gZeldasCourtyardGanonCs`: Ganondorf kneels,
   text 0x7009); the plan; **Zelda's Letter** (`GI_ZELDAS_LETTER`,
   `EVENTCHKINF_OBTAINED_ZELDAS_LETTER`).
4. **Impa** (`Demo_Im`, waiting for the letter's flag) appears; Link walks near her (within 130):
   `gZeldasCourtyardLullabyCs` (1,980 frames), `Item_Give(ITEM_SONG_LULLABY)` at once, and the
   script teaches the song through its ocarina texts: `OCARINA_ACTION_TEACH_LULLABY` (0x0A, frames
   280 to 535: Impa whistles it, `OCARINA_INSTRUMENT_WHISTLE`), then
   `OCARINA_ACTION_PLAYBACK_LULLABY` (0x17: Link plays it back, 0x88B on a mistake). The script
   holds its frame while the message system is busy (the port has the hold; it logs the ocarina
   action today). Its terminator (`CS_DEST_HYRULE_FIELD_FROM_ZELDAS_COURTYARD`, frame 875) enters
   `ENTR_HYRULE_FIELD_0` with `CS_INDEX_8` and a slow white circle wipe.
5. **Hyrule Field's layer 12** (`gHyruleFieldImpaCs`): Impa (`Demo_Im` type 4) talks
   (0x2060, 0x204F, 0x2062, 0x2063), throws a Deku nut (`En_Arrow`'s `ARROW_CS_NUT`, ported) and is
   gone; its terminator enters `ENTR_HYRULE_FIELD_17`, fading to black.

**`z_sram.c` 572:** loading a save with the letter but without the lullaby turns the letter back
into the Cucco and clears its flag (ported with saving, GAME-05 milestone 5c: to check).

| Actor | Lines | Ported | Reached | What |
|---|---|---|---|---|
| `En_Zl4` | 1,341 + 186 | no | yes, whole | Above. Later visits: 0x703C / 0x703D. Face: 7 eyes, 4 mouths |
| `Demo_Im` | 1,255 + 222 | no | types 3 (the whistle), 4 (the field), 5 (the courtyard): about 450 lines and the shared 300 | Impa. Also the Chamber of Sages' Shadow Medallion and Ganon's sealing, the credits |
| `En_Ta` | 1,344 | no | the castle's (about 150) and the Kakariko guest house's (killed for a child); the shared wake-up (46) and draw (390) | Talon. Lon Lon Ranch's (the cucco game, the milk) is about 670 |
| `En_Heishi1` | 526 | no | yes, whole | The courtyard's patrols |
| `En_Heishi2` | 885 | no | types 2 (the castle's gate), 5 (Kakariko's gate), 6 (the window's guards); 9 isn't placed | The guards that open gates |
| `En_Heishi3` | 271 | no | yes, whole | The castle grounds' sentries |
| `Bg_Spot15_Rrbox` | 381 | no | yes | The milk crates |
| `Bg_Spot15_Saku` | 114 | no | yes | Lon Lon's fence by the gate |
| `En_Light` | 208 | no | the castle, the shops, the graves | Fires (torches' flames, candles) |
| `Obj_Tsubo` | 359 | no | the castle (2), Kakariko (11), the guard house (44), the graves | Pots: broken by a sword, a roll or a throw; lifted |
| `En_Owl`, `Door_Ana`, `Obj_Bombiwa`, `En_Wood02`, `En_Kanban`, `En_Ishi`, `En_Kusa`, `En_Wonder_Item`, `En_Wonder_Talk2`, `En_Viewer`, `Demo_Effect`, `Demo_Kankyo` | | yes | | The castle's owl (`OWL_HYRULE_CASTLE`) is ported |

### 3. Kakariko

**The village** (`spot01`). `gKakarikoVillageIntroCs` plays on the first arrival
(`EVENTCHKINF_A3`; the night route reached it in GAME-06). Its bg cameras are `START1` (ported)
with `NORMAL3` (`Camera_Jump3`, about 200 lines: not ported) and `DUNGEON1`, `NORMAL1`.

By day: Anju (`En_Niw_Lady`) and her seven cuccos (`En_Niw`), the man on the roof (`En_Ani`), the
carpenters (`En_Daiku_Kakariko`) and their boss (`En_Toryo`), the gate guard (`En_Heishi2` type 5)
at Death Mountain's gate (`Bg_Gate_Shutter`), two guards (`En_Heishi4`), the windmill
(`Bg_Spot01_Fusya`), the well (`Bg_Spot01_Idohashira`, `_Idomizu`), the posters and the
unfinished gallery (`Bg_Spot01_Objects2`, which spawns a third carpenter), 11 pots, a crate
with a cucco in it. By night: the carpenter's son (`En_Hs2`), four Gold Skulltulas (`En_Sw`,
ported), no cuccos and no Anju (she's in Impa's house).

- **Anju's cuccos:** she counts the cuccos in her pen every frame (|x - 330| < 90, |z - 1610| <
  190) and keeps a saved bit per cucco (`INFTABLE_199` to `_19F`). Seven penned: the Empty Bottle
  (`GI_BOTTLE_EMPTY`, `ITEMGETINF_0C`). This needs **Player's lift, carry and throw**: an `En_Niw`
  offers itself (`Actor_OfferCarry`), A lifts it (`Player_ActionHandler_2`, `func_8083A0F4`,
  `Player_Action_80846050`), Link carries it (`Player_UpperAction_CarryActor`, with **the cucco's
  glide**: a gentle fall and no fall damage), and A throws or puts it down (`Player_Action_80846578`,
  `_808464B0`). The crate's cucco is lifted through `Actor_OfferGetItem(GI_NONE)`. The same
  actions lift pots.
- **The cuccos' revenge:** hit one 5 to 8 times and it crows (one-point cutscene 2290) and calls
  attack cuccos (`En_Attack_Niw`, 427 lines), one every 10 frames up to 7, diving at Link.
- **The gate guard** takes Zelda's Letter (`EXCH_ITEM_ZELDAS_LETTER`): the gate opens with a sub
  camera, `INFTABLE_76` is set (which also opens the mask shop). This needs **Player's item shown
  in a talk** (the exchange items: `Player_ActionHandler_13`'s trade items, not ported), as
  Talon's Cucco does.
- **The man on the roof** (`En_Ani`) gives a piece of heart when Link reaches the roof behind him
  (the watchtower, or a cucco's glide).

**The graveyard** (`spot02`; `gGraveyardIntroCs` on the first entry, `EVENTCHKINF_B6`). By day:
the graveyard kid (`En_Cs`, on a path; he buys the Spooky Mask), a gossip stone, the gravestones
(`Bg_Haka`, ported), the inscriptions (`En_Wonder_Talk`), the soft soil (`Obj_Bean`, ported) and a
bug patch (`Obj_Makekinsuta`, 72: bugs released there make a Gold Skulltula), the royal tomb's
gravestone (`Bg_Spot02_Objects` type 2) and its lullaby spot (`En_Okarina_Tag` type 6). By night
(18:00 to 21:00): Dampé (`En_Tk`, 754) and his dig spots (`En_It`, 73): ten rupees for a dig, a
random drop, a piece of heart on the first purple-rupee roll.
- **The royal tomb** opens to Zelda's Lullaby (`gGraveyardTombOpeningChildCs`, 1,637 frames, its
  gravestone blown up, `EVENTCHKINF_1D`). Inside (`hakaana_ouke`): ReDeads (`En_Rd`, 1,037), Keese
  (`En_Firefly`, ported), a shutter, and the wall that teaches **the Sun's Song**
  (`En_Okarina_Tag` type 5 → `gSunSongGraveSunSongTeachCs`: teach 0x0B and playback 0x18 by the
  script's ocarina texts, then `gSunSongGraveSunSongTeachPart2Cs` and the graveyard's part 2).
  The Sun's Song's fairy spot there is `Shot_Sun` (BACKLOG #36; Hyrule Field has another).
- **The graves:** the ReDead grave (`hakaana`: an `En_Rd` and a chest) and the fairy fountain
  grave (`hakaana2`: `Bg_Breakwall` (333) and a fairy).

**The interiors**, for a child:

| Place | What a child meets | Actors | Lines |
|---|---|---|---|
| Impa's house (`labo`) | a townsman (`En_Hy` type 17), Anju by night, the cow (Epona's Song: milk), a piece of heart in the cow's pen | `En_Hy`, `En_Niw_Lady`, `En_Cow` (447) | 447 |
| The guest house (`kakariko`) | a townswoman by day (`En_Hy` type 18); the carpenters and their boss by night. Talon (`En_Ta` type 1) is killed for a child | `En_Hy`, `En_Daiku_Kakariko`, `En_Toryo`, `En_Ta` | |
| The potion shop (`drag`) | a carpenter (the shopkeeper, type 1, is killed for a child) | `En_Daiku_Kakariko`, `En_Ossan` | |
| The bazaar (`shop1`) | the shop shared with Castle Town (its Kakariko entrance and text 0x9D) | `En_Ossan` type 4, `En_Hy` type 20, `En_Go2` | |
| The House of Skulltula (`kinsuta`) | the cursed family (`En_Ssh`, 905: hanging, spinning when hit) and the cured (`En_Sth`, 421: the rewards for 10 to 100 tokens: wallets, the Stone of Agony, Bombchus, a piece of heart, a gold rupee) | `En_Ssh`, `En_Sth` | 1,326 |
| The windmill (`hakasitarelay` room 6) | the windmill man (`En_Fu`, 326: "Go around!"), its gears (`Bg_Relay_Objects`, 221); the Song of Storms' spot (`En_Okarina_Tag` type 2: the child plays it to drain the well; the adult learns it) | | 547 |
| Dampé's hut (`hut`) | an inscription | `En_Wonder_Talk` | |
| Granny's potion shop (`mahouya`) | the closed door's text outside; inside (adult) | `En_Ds` (292) | |
| The back alley house (`kakariko3`) | a townsman (`En_Hy` type 13), pots | | |

### 4. The ocarina

A child gets the Fairy Ocarina on the Lost Woods' bridge (GAME-06 milestone 2), and can't play
it yet: Player's ocarina action and the message system's ocarina modes aren't ported. Zelda's
Lullaby needs the teach and playback modes (from a cutscene's text), the royal tomb and the Sun's
Song need the check modes and free play, and `Shot_Sun` (BACKLOG #36), the gossip stones' fairies,
Epona's Song (the cow), Saria's Song (Navi's text) and later every song read what was played.

| Part | C | Lines | What |
|---|---|---|---|
| Player | `z_player.c`: `Player_Action_8084E3C4` (13786), `Player_ActionHandler_13`'s ocarina (6140), the ocarina after a talk (`func_8084B4D4`), the warp's respawn | about 100 | Taking it out (`okarina_start`), the turnaround camera (`CAM_ITEM_TYPE_90` / `91`: `Camera_KeepOn4`, ported), free play or an actor's check (the "listener" protocol: `PLAYER_STATE2_23` to `_25`), the end (talk to the actor, to Navi, or put it away), the warp songs' respawn |
| The world held | `z_actor.c` 2444 | 3 | While Link plays, only actors with `ACTOR_FLAG_UPDATE_DURING_OCARINA` update |
| The message system | `z_message.c`: `Message_StartOcarinaImpl` (2805), `Message_HandleOcarina`, 42 of the 56 `MSGMODE`s in `Message_DrawMain` (3135 to 4013), the staff and its notes, `Message_Update`'s ocarina parts | about 1,270 (980 without the scarecrow's songs, the Skull Kids' memory game and the frogs) | Free play, the check modes (`OCARINA_ACTION_CHECK_*`), teaching (`TEACH_*`: the instrument by teacher) and playback (`PLAYBACK_*`, `Item_Give` of the song), "You played X" (0x893 + song), the warp prompt, the fanfares (`NA_BGM_OCA_*`), the treble clef and the note icons, `ocarinaMode` and `lastPlayedSong` |
| The audio | `src/audio/game/general.c` 792 to 2318 | about 1,530 (about 1,130 without the recording and the memory game) | `AudioOcarina_*`: the pad read (`ReadControllerInput`), the notes by button, R and Z a semitone, the stick's bend (two semitones) and vibrato, the song tables (`sOcarinaSongNotes`, 14 songs), song recognition, the teacher's playback, the scarecrow's recording. The notes are a sound effect, `NA_SE_OC_OCARINA` on `SFX_CHANNEL_OCARINA`, its instrument, pitch and vibrato through the channel's IO ports 5 to 7 |
| The HUD | `z_parameter.c` | about 130 | `HUD_VISIBILITY_B_ALT` (B "Stop"), the scene's ocarina restrictions, the songs' `Item_Give` |
| The Sun's Song | `z_parameter.c` 4359 (`Interface_Update`) | 63 | Time at 400 a frame to 6:30 or 18:00, or the scene entered again at the other half of the day |
| The cutscenes' ocarina text | `z_demo.c` `CutsceneCmd_Text` 1754 | 76 (the hold is ported) | `CS_TEXT_OCARINA_ACTION`: the lullaby's and the Sun's Song's teaching |
| The songs' effects | `Oceff_Wipe` (155, the lullaby, the Song of Time), `Oceff_Spot` (195, the Sun's Song), `Oceff_Wipe2` (143, Epona's), `Oceff_Wipe3` (138, Saria's), `Oceff_Storm` (206) with `En_Okarina_Effect` (142, the storm), `Oceff_Wipe4` (144, the scarecrow's) | 1,123 | The light each song spawns (`MSGMODE_SONG_PLAYED`) |
| The spots | `En_Okarina_Tag` (355 + 138 of its cutscenes), `Shot_Sun` (225) | 718 | The lullaby spots, the royal tomb, the windmill, the Door of Time, the Sun's Song's wall; the fairy spots |

**About 3,200 lines for the system, 1,840 for the effects and spots.** The audio side is new
ground for the port's sound effect channels (IO ports read by a channel's script); the rest is
the message box's states, which the port has for texts and choices.

### 5. Systems the areas need

| System | Lines | Port today | Needed by |
|---|---|---|---|
| Player's item shown in a talk (the exchange items: `Player_ActionHandler_13`'s trade items, the held-up item) | about 150 | logged | Talon (the Cucco), the Kakariko guard (the letter); later the trade sequence |
| Player's lift, carry and throw, the cucco's glide | about 400 | logged | Anju's cuccos, the pots, the crate's cucco; later bombs, rocks |
| `Camera_Special6` (the guards' courtyard) | about 100 | no | the courtyard |
| `Camera_Fixed1` (the market's balcony; Bombchu Bowling) | about 70 | no | the market |
| `Camera_Jump3` (`NORMAL3`) | about 200 | no: falls back to `Camera_Normal1` | Kakariko, the castle |
| `TransitionCircle` (`z_fbdemo_circle.c`) | 227 | drawn as a black fade | the guards' starburst, the courtyard's white wipe to the field, and many exits |
| The dawn's counters and the egg's hatch (`TIMESEQ_DAY_BEGIN_SFX`'s rest, `Play_Init`'s `NEXT_TIME_DAY`) | about 30 | the music's half | the Weird Egg |
| A dog following Link across scenes (`dogParams`, Player's respawn of it) | about 25 | no | the dog lady's quest |
| `SCENE_CAM_TYPE_FIXED_MARKET`'s message box place | small | no | the market |
| The culling volume (`Actor_CullingVolumeTest`, BACKLOG #32) | about 60 | no | the market's and Kakariko's crowds (everything updates and draws today); Hyrule Field's trees |
| `En_Bom` (BACKLOG #33) | 409 | no | not this phase: bombs come with Dodongo's Cavern; the Peahat's explosion, the hidden grottos and the castle's Great Fairy rock wait for it |

## The proposal

**Six milestones, in the story's order, the ocarina before Zelda** (each one commit, as GAME-06's):

1. **Castle Town:** the entrance, the market, the back alley and the Temple of Time's outside by
   day and by night, and the Temple of Time's altar. `En_Hy`, `En_Heishi4`, `En_Dog` (with its
   following), `En_Ma1`, `En_Mu`, `En_Niw_Girl`, `En_Tg`, `En_Gs`, `Obj_Mure` with `En_Butte`;
   the Temple's `En_Wonder_Talk`, `Bg_Toki_Hikari`, `Bg_Mjin`; `Camera_Fixed1`; the market's
   message box; BACKLOG #1 (the 16:9 edges of the fixed pictures); **the culling volume
   (#32)** first, since the market is the first crowd. About 6,000 lines.
   **Exit:** from Castle Town's entrance by day, through the market (Malon's talk, a townsman's),
   to the castle's road, where `gHyruleCastleIntroCs` plays: `Route::Market`. A second short
   run by night (a dog following Link out of the back alley) if the first is long.
2. **Hyrule Castle's grounds:** `En_Ma1`'s egg, the egg's hatch at dawn, Player's exchange items,
   `En_Ta` (the castle's part), `Bg_Spot15_Rrbox`, `Bg_Spot15_Saku`, `En_Heishi2`, `En_Heishi3`
   with the throw-out, `En_Light`, `Obj_Tsubo` (broken; lifted from milestone 5),
   `TransitionCircle`, `Camera_Jump3`. About 3,000 lines.
   **Exit:** from the castle's road with the Cucco (a preset), Talon woken and run off, two crates
   pushed into the moat, up and through the drain into the guards' courtyard: `Route::Castle`.
3. **The ocarina:** the system above, whole but for the scarecrow's songs, the memory game and the
   frogs (logged); `Oceff_Wipe`, `_Wipe2`, `_Wipe3`, `_Spot`; `Shot_Sun` (BACKLOG #36); the Sun's
   Song's time; the cutscenes' ocarina text. About 4,000 lines.
   **Exit:** in Hyrule Field with the songs owned (a preset): the ocarina out, Saria's Song (Navi
   talks), the Sun's Song (dusk to dawn), and the Song of Storms at `Shot_Sun`'s spot (a big fairy):
   `Route::Ocarina`; the teach and playback modes tested by a test that plays them from a script's
   text.
4. **The guards' courtyard and Zelda:** `Camera_Special6`, `En_Heishi1` with its search balls,
   `En_Zl4`, the legend's cutscene map, the letter, `Demo_Im` (the courtyard, the whistle, the
   field), Zelda's Lullaby taught and played back, Hyrule Field's layer 12. About 4,000 lines.
   **Exit:** from the guards' courtyard's drain (a preset after Talon), past the guards, Zelda's
   talk through the legend and the window to the letter, Impa's lullaby (one wrong note, then
   right), the field's goodbye to `ENTR_HYRULE_FIELD_17`: `Route::Zelda`.
5. **Kakariko:** the village by day and by night, Player's lift, carry and throw with the cucco's
   glide, `En_Niw` with `En_Attack_Niw`, `En_Niw_Lady`, `En_Ani`, `En_Daiku_Kakariko`, `En_Toryo`,
   `En_Hs2`, the gate (`Bg_Gate_Shutter`, the guard taking the letter), the windmill and the well,
   and the interiors: Impa's house (`En_Cow`), the guest house, the potion shop, the House of
   Skulltula (`En_Ssh`, `En_Sth`), the windmill (`En_Fu`, `Bg_Relay_Objects`). About 7,000 lines.
   **Exit:** from Kakariko's entrance by day with the letter (a preset): three cuccos carried into
   Anju's pen (one glided off a roof), the letter shown to the guard and the gate opened:
   `Route::Kakariko`.
6. **The graveyard:** `En_Cs`, `En_Tk` and `En_It` (Dampé's dig at night), `Obj_Makekinsuta`, the
   royal tomb (`En_Okarina_Tag`, `Bg_Spot02_Objects`, the opening cutscene), its ReDeads (`En_Rd`)
   and the Sun's Song taught, the graves (`Bg_Breakwall`). About 3,500 lines.
   **Exit:** at 18:00 with the lullaby (a preset): a dig with Dampé, the lullaby at the royal
   tomb, past the ReDeads (frozen by the Sun's Song after it's learned) to the song's wall:
   `Route::Graveyard`.

Milestones 1, 2, 4 and 5 are each about the size of GAME-06's milestone 4 or bigger; 1, 4 and 5
may split in two (people and places) when they're built, as GAME-05's did.

**Not in the phase** (left as placeholders, BACKLOG rows): the shops' other shopkeepers and the
games (the potion shop, the bazaar, the mask shop, the chest game, the shooting gallery: about
4,000 lines with their items), which can be a seventh milestone or wait for the breadth pass;
Bombchu Bowling and the Bombchu shop (closed to a child before Dodongo's Cavern); `En_Bom`
(BACKLOG #33) with the bombs; Lon Lon Ranch (with Epona's phase).

**The choices** (the user's in the status above):
- the split above, or another (Kakariko first, since the night route reaches it; the castle in
  one milestone with the ocarina folded into Zelda's);
- which actors are ported whole and which only for the parts reached. Proposed: whole, except
  `En_Ta` (the castle's and Kakariko's parts; the ranch's later), `Demo_Im` (types 3, 4 and 5; the
  Chamber of Sages' and the credits' later), the killed-for-a-child actors (`En_Xc`, `Demo_6K`,
  `Bg_Toki_Swd`, `En_Go2`: their inits' checks only, then placeholders), and the message system's
  and the audio's scarecrow, memory game and frog parts;
- whether the ocarina is its own milestone (proposed: yes, before Zelda);
- the candidates: the culling volume (#32) at the start of milestone 1 (proposed), `En_Bom` (#33)
  later (proposed).

## Milestone 1: Castle Town

**Answer:** done. Castle Town is full by day:
- Malon sings by the fountain and talks of her dad gone to the castle;
- the townsfolk stand, fidget, pace and talk; the two by the stall haggle through five texts;
- the dancing couple spins, the cucco girl chases her cucco round the market, and a dog wanders;
- three butterflies flutter over the planter; the guards stand at the entrance and, by night,
  in the market.

Beside the Temple of Time stand four gossip stones; struck, they wobble, twist, squash or spin. Inside,
the altar has its text, the windows their light, and the warp pad its Light Medallion. At 16:9
the game's 4:3 frame is shown with black beside it (BACKLOG #1; the game culls at its 4:3 edges). The culling volume (ADR 0058)
came first. The exit holds. Its run is the golden `market`, with `market_townsman`,
`market_malon` and `market_castle`.

The pack is still format 28 in `out/data25`, re-imported for the new bakes; builds in
`target/game26`. Decisions are in [ADR 0058](adr/0058-the-culling-volume.md) and
[ADR 0059](adr/0059-castle-towns-actors.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 104 to 106):
- `test-market.bat`: the milestone's tests;
- `game-market.bat`: Castle Town's entrance by day (`deku-tree-dead`); `night` (20:00),
  `temple` (the Temple of Time's outside), `altar` (inside) or `alley` (the back alley);
- `sandbox-market.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** milestone 1 is done; 2 (Hyrule Castle's grounds) is next.

### What was built

**The culling volume** (ADR 0058): `Actor_CullingVolumeTest` where `Actor_DrawAll` makes it, every
ported actor's volume from its C, `viewProjectionMtxF` taken at `View_Apply`'s place with
`lightCtx.zFar` (BACKLOG #32).

**The townsfolk**, whole:
- `En_Hy` (`oot_actors::en_hy`): the 21 types, their heads, colours and eyes as bakes, the walks
  and paces, the dog lady's reward and the beggar's purchases (`Player_UpdateBottleHeld` logged);
- `En_Heishi4`: the talking guards (Kakariko's two, the market's by night, the dying guard after
  Zelda's escape);
- `En_Dog`: the dogs, the lost dog that follows Link (`Player_Init` and `Player_Update`'s
  following dog, `dogParams`);
- `En_Mu`: the haggling townsfolk, their five texts in turn (`EnMu_Interact`, `eventInf[2]`);
- `En_Tg`: the dancing couple;
- `En_Ma1`: child Malon: where she is by the story (`EnMa1_ShouldSpawn`), her texts and flags,
  the Weird Egg (`GI_WEIRD_EGG`), her song's voice (`Audio_ToggleMalonSinging`,
  `Audio_UpdateMalonSinging` in `oot_game::audio`), Epona's Song's lesson (the ocarina's calls
  logged);
- `En_Niw_Girl` and `En_Niw`: the girl's chase, her cucco on her path; the cucco whole (its kinds,
  its flight from Link, the strikes and the attack's call, its feathers, carried and thrown with
  Player's lift logged), one-point camera 2290.

**The Temple of Time's outside and altar:** `En_Gs` (the gossip stones: talk, the Mask of Truth's
hints, the songs' fairies with the ocarina logged, the five reactions to blows, the bomb's
launch), `En_Wonder_Talk`, `Bg_Toki_Hikari` (the windows, the Door of Time's beam), `Bg_Mjin`
(the warp pads).

**The butterflies:** `Obj_Mure` (the groups: alive near the camera, their followers) and
`En_Butte` (the flight, the fairy for a Deku Stick held still).

**Elsewhere:**
- `Camera_Fixed1` (the market's balcony, `oot_game::camera`);
- the game's 4:3 frame at 16:9 (`DrawLists::pillarbox` on every play state's frame, BACKLOG #1);
- bakes: a list from an offset (`symbol+0x50`), a material list bound to a segment to run first;
- Player's interface: `held_item_action`, `melee_weapon_tip`, `set_exchange_item_id`,
  `current_mask`;
- `oot_game::path` (`Path_GetByIndex`, `Path_OrientAndGetDistSq`, `Path_CopyLastPoint`), the NPC
  tracking helpers (`Actor_TrackNone`, `_TrackPoint`, `_TrackPlayer`), `eventInf`;
- `Route::Market` (`--script market`), with `Who::Hylian` and `Who::Malon`.

### Results

**Tests.** `cargo test --release --workspace` (`target/game26`, `OOT_DATA_DIR=out/data25`): 832
passed, 0 failed, 1 ignored (812 after the culling volume). New, with their expectations from the C:
- **`oot_actors --test market`** (13):
  - Malon in the market until she has been at the castle: the castle's first visit sets
    `INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE` without her, the second has her, and the market
    then hasn't; Talon's return takes her out of it.
  - Malon's texts by the story: 0x2041, 0x2042, 0x2043 (the egg), 0x2044, 0x2047, 0x2048, 0x2049,
    0x204A with Epona's Song.
  - Malon's voice off on the frame a talk is accepted, still singing; the idle animation the next
    frame; singing again after.
  - The haggling townsfolk: all five texts before any again, and never the last one twice.
  - The dancing couple: 0x800 a frame; 0x7025 and 0x7026 by turns.
  - The cucco girl: her path 2, her cucco (type 0xA) 50 ahead and immovable, given path 3 and its
    point count once she's close, running it.
  - A struck cucco: one strike of its 5 to 8 counted, 20 feathers the next frame, its flight
    (speed 4).
  - A gossip stone struck by the sword: the wobble, then text 0x71B1.
  - The market's butterflies: `Obj_Mure` 0x3324 (3, pattern 3, kind 1, butterflies) spawns three
    when in view, the first the fairy one (culling scale 200, the others 20).
  - A butterfly's init: params -1 to 0, the volume (700, 20, 600), `shape.rot.x` -0x2320, the
    first flight 1 or 2 and its timer.
  - The Temple: the warp pad's collision, the altar's text 0x7040 (range 80), the beam waiting for
    the room's `drawParams[1]`, growing 0.05 a frame to 1.
  - `Camera_Fixed1` on the balcony (-450, 160, 75): the eye at the bg camera's (0, 64, 0), the yaw
    its rotation's (-12743).
  - The exit's run (below).
- Unit tests: `En_Ma1`'s damage table and transition, `En_Niw`'s float-to-`s16`, `En_Gs`'s
  `DEG_TO_BINANG2` and damage table.

**The exit run** (`Route::Market`, `--script market`, `ENTR_MARKET_ENTRANCE_DAY_0`,
`deku-tree-dead`): `market` 103, `townsman` 435, `malon` 920, `castle_road` 1210. Texts: the
red-shirted townsman's 0x701F and 0x7020, Malon's 0x2041 and 0x2042; the castle's intro
(`gHyruleCastleIntroCs`) from frame 1064 to 1200. Flags at the end:
`INFTABLE_TALKED_TO_MALON_FIRST_TIME`, `EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME`,
`EVENTCHKINF_A5`, `INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE`.

**The goldens.** After the culling volume (146 hashes, proved in three builds, golden/README.md),
against its 146: 144 the same bytes. `field/shot.png` changed for the entrance's guard and the
pillarbox, `field_night/shot.png` for Kakariko's gate guard (`En_Heishi4`); each cause was proved by
taking it out. New: `market` (the trace and the end), `market_townsman` (frame 380),
`market_malon` (860), `market_castle` (1120); each the same bytes over two runs. Logged in
[golden/README.md](../golden/README.md): 151 hashes, 112 cases. After the hand test, the 4:3 frame
for every play state re-recorded 69 renders (the middle 4:3 the same bytes, black beside it;
proved by the old rule giving back all 151).

### Decisions

- **[ADR 0058](adr/0058-the-culling-volume.md):** the culling volume and the view's place.
- **[ADR 0059](adr/0059-castle-towns-actors.md):** Castle Town's actors whole with their gaps
  logged; the pillarbox; two bake forms; overlay statics; Player's held item and sword tip;
  `Camera_Fixed1` and 2290; the Temple's room 0 left for the Door of Time.

### Known gaps

- **The Temple's room 0** (behind the Door of Time) stays placeholders: a child can't reach it.
  The plan's "killed for a child" was wrong for `Demo_6K` and `Bg_Toki_Swd` (BACKLOG #37).
- **Unported spawns** (`En_Attack_Niw`, `En_Poh`, `En_Fish`, `En_Insect`) are placeholders
  (BACKLOG #38); **masks** (#39); **splashes, ripples and the stones' blast** (#40); **the
  beggar's bottle** (#41); **Player's lift** (#4: the cuccos can't be carried yet).
- **The ocarina's parts** (Malon's lesson, the stones' songs) wait for milestone 3.
- **`En_Okarina_Tag`** at the Door of Time is a placeholder until milestone 3.
- **Shadows** stay the circle stand-in.
- A second run by night (a dog following Link out of the back alley) wasn't needed: the day's run
  is short. The night is in the hand tests.

### Fixes found while building

- **After the hand test (2026-10-12):** the new actors were invisible in the game: my import had
  written the default pack, not `out\data25`, which the scripts read (re-imported there). And the
  townsfolk vanished at the screen's sides at 16:9: the game culls through its 4:3 view, so every
  play state now shows the 4:3 frame (ADR 0059).
- **The haze outside the Temple of Time** is the game's own: the scene's light setting 0, grey fog
  from 970 to 3000, which its floors (light setting 31, folded to 0) keep. Looking into it showed
  Player's floor light setting (`Environment_ChangeLightSetting`) wasn't ported: it is now, and the
  Temple of Time's inside takes its floors' settings 2 and 3 (test
  `player_takes_the_light_setting_of_the_floor_he_stands_on`; the goldens unchanged).

- **`En_Dog`'s colour** was baked on segment 13, which holds the skeleton's matrices: the import
  failed; now segment 12, as a prelude.
- **Constants checked against the headers:** `EXCH_ITEM_*`, `ITEM_BOTTLE_BUG`, `GI_RUPEE_BLUE`,
  `OBJECT_DOG` (0x016B).

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-market.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-market.bat
```

### By hand

`game-market.bat` (or menu 105). Space A, WASD the stick, E the sword.
1. **The entrance:** black bars beside the 4:3 frame; a guard by the guard house's door.
   North into the market.
2. **The market:** Malon by the fountain, singing; her song's voice fades with the camera's
   distance and stops while she talks: "Dad went to the castle...". The cucco girl chases her
   cucco round the north side and stops to jump; talk to her while she jumps.
3. **The townsfolk:** the two at the stall (south-west) haggle: talk to them five times, a new
   text each time. The red- and blue-shirted men argue; the dancing couple spins by the tree and
   stops to talk; a white dog wanders.
4. **The balcony:** up onto the west walkway (-450, 160, 75): the camera turns to look out from the
   market's centre.
5. **The castle's road:** north out of the market: the castle's intro, its title card.
6. **Night:** `game-market.bat night`: the guards ("Kids shouldn't be out at night"); in the back
   alley a lost dog follows Link.
7. **The Temple:** `game-market.bat temple`: four gossip stones; hit one with the sword (a wobble
   and "it doesn't seem to want to talk"). `game-market.bat altar`: the intro the first time, the
   light through the windows, the warp pad's medallion, the altar's text.
- **What to report:** Malon's look (her eyes, her mouth, her song), the cucco's flight and
  feathers, the butterflies, the stones' reactions, the windows' light, and whether anything
  stands where it shouldn't (a placeholder marker means an actor not ported).
