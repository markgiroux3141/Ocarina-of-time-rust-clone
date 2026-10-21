# GAME-10: Zora's River, Zora's Domain, Lake Hylia, Zora's Fountain and Jabu-Jabu's Belly

**Goal:** Phase 11 of the [roadmap](ROADMAP.md): child Link with the Kokiri Emerald and the Goron's
Ruby up Zora's River (the waterfall opened by Zelda's Lullaby) into Zora's Domain, the silver scale
from the diving game, Ruto's letter from the bottom of Lake Hylia, King Zora moved aside, a fish
for Lord Jabu-Jabu, Jabu-Jabu's Belly (Master Quest) and its boomerang, Barinade, Ruto's warp and
the Zora's Sapphire; then, with the three stones, Zelda's escape, the Ocarina of Time in the moat
and the Song of Time. Exit (ROADMAP's outline): the Zora's Sapphire, then the Ocarina of Time.

**Status:** surveyed, proposed and decided (2026-10-20; the user took every recommendation: the
last section). The phase starts from the `fairy-magic` preset (GAME-09's exit review).

| # | Milestone | Status |
|---|---|---|
| 1 | Water and bottles | done (2026-10-20) |
| 2 | Zora's River and Zora's Domain to the silver scale | done (2026-10-21) |
| 3 | Ruto's letter, King Zora and Lord Jabu-Jabu | done (2026-10-21) |
| 4 | Jabu-Jabu to the boomerang | done (2026-10-21) |
| 5 | Jabu-Jabu to Barinade | |
| 6 | Barinade and the Zora's Sapphire (the exit's first part) | |
| 7 | The Ocarina of Time and the Song of Time (the exit's second part) | |
| 8 | The rest of Jabu-Jabu | |
| 9 | The frogs and the scarecrows | |

## The survey

Checked against the decomp (52a510f) and the pack (`out/data26`), not against earlier notes. Line
numbers are the decomp's; `ootx scene-info` was run on every layer of the phase's scenes (and
Hyrule Field's and the Temple of Time's for the moat), `ootx cutscene` on the scripts named.
`baseroms/gc-eu-mq-dbg/config.yml` uses `bdan_mq.xml` for the dungeon and `bdan_boss.xml` for
Barinade's room.

### The scenes and their layers

The entrance intros (`sEntranceCutsceneTable`, `z_demo.c:122-157`) play on whatever layer is loaded:
`ENTR_ZORAS_DOMAIN_0` (`EVENTCHKINF_A4`, `gZorasDomainIntroCs`), `ENTR_LAKE_HYLIA_0` (`_B1`,
`gLakeHyliaIntroCs`), `ENTR_ZORAS_FOUNTAIN_2` (`_B7`, `gZorasFountainIntroCs`), `ENTR_JABU_JABU_0`
(`_B5`, `gJabuIntroCs`: 300 frames, camera only, the title card at 10), all for both ages;
`ENTR_HYRULE_FIELD_16` (`_A9`, `gHyruleFieldGetOoTCs`) a child's only. Zora's River has none.

| Scene | Layer | Cutscene | Who meets it |
|---|---|---|---|
| `spot03` (Zora's River) | 0, 1 | | a child (layer 1's header is 0: layer 0) |
| | 2, 3 | | an adult (Phase 16) |
| `spot07` (Zora's Domain) | 0, 1 | | a child (layer 1 = layer 0) |
| | 2, 3 | | an adult (Phase 16) |
| | 4 | `gZorasDomainCreditsCs` | the credits (Phase 19; `gLakeHyliaCreditsCs`' terminator 61 sets `CS_INDEX_0`, `z_demo.c:1057-1061`) |
| `spot08` (Zora's Fountain) | 0 | | a child by day |
| | 1 | | a child by night: its own header (0x4550), layer 0's actors plus `En_Sw` 0xB204 |
| | 2, 3 | | an adult (Phase 16) |
| | 4 | `gZorasFountainSapphireCs` | **this phase**: Ruto's blue warp (`z_door_warp1.c:622-629`, `CS_INDEX_0`) |
| | 5 | `gZorasFountainUnusedJabuCs` | nobody: nothing sets `CS_INDEX_1` for the fountain (the map select only) |
| | 6 | `gZorasFountainCreditsUnusedCs` | nobody: `CS_DEST_ZORAS_FOUNTAIN_CREDITS` ends no script |
| `spot06` (Lake Hylia) | 0, 1 | | a child (layer 1 = layer 0) |
| | 2, 3 | | an adult (Phase 16) |
| | 4 | `gLakeHyliaRestoredCs` | Phase 16 (`gWaterMedallionCs`' terminator 21, `z_demo.c:792-797`) |
| | 5 | `gLakeHyliaCreditsCs` | the credits (Phase 19; `gGoronCityCreditsCs`' terminator 59, `z_demo.c:1043-1048`) |
| `hylia_labo` (the lab) | 0 to 3 | | everyone (one set of placements) |
| `bdan` (Jabu-Jabu, MQ) | 0 to 3 | | everyone (one set) |
| | 4 | `gJabuRutoObtainingSapphireUnusedCs` | nobody: an unused layer shaped like the original game's |
| `bdan_boss` (Barinade) | 0 to 3 | | everyone (one set) |
| `spot00` (Hyrule Field) | 1 | | **a child with the three stones** (`z_play.c:426-434` forces layer 1 by day and night; layer 0 otherwise): adds `Item_Ocarina` 0x0003 and `En_Mm`, drops the four owls |
| | 5 | `gHyruleFieldZeldaEscapeCs` | **this phase**: the drawbridge sets `CS_INDEX_1` (`z_bg_spot00_hanebasi.c:231-236`) |
| `tokinoma` (Temple of Time) | 11 | `gTempleOfTimeSongOfTimeCs` | **this phase**: `CS_INDEX_7` from `gHyruleFieldZeldaSongOfTimeCs`' terminator 52 (`z_demo.c:990-995`) |

**Corrections to `scripts/coverage.py` (proposed):**
- `SCENES`: `spot07` gains `{4: 19}`; `spot06` `{5: 19}` (4 stays 16); `spot08` `{5: None, 6: None}`;
  `tokinoma` `{11: 11}`; `spot00` becomes `{1: 11, 5: 11, 6: 19, 7: 20, 8: None, 9: 19, 10: 13, 11:
  19, 12: 8}`, and its comment is wrong: 8 is `gHyruleFieldGetOoTCs` (the map select's only: in
  play it runs on layer 1 by the entrance table), 9 the credits, 11 before the credits.
- `ACTORS`: `"Bg_Spot08_Iceblock": 16` (placed in the fountain's child layers as 0x0214 but killed
  for a child, `z_bg_spot08_iceblock.c:325-328`): **the only actor whose first phase moves.**
  `"Item_Ocarina": 11` becomes redundant with `spot00`'s layer 1 (harmless).
- Side effect: with `spot08`'s 5 and 6 at `None`, **`En_Fish` loses its only placements** and moves
  to "only spawned" (`Obj_Mure` type 2 and Player's bottle spawn it). It's still this phase's
  (Lord Jabu-Jabu's bait).

### The route a child takes

From `fairy-magic` (in Kakariko after the owl's flight). What the save lacks for it, the route
gives: the silver scale, Ruto's letter (in its own bottle), the empty bottle (King Zora's), the
fish, the boomerang. **The diving game costs 20 rupees; the save has 16.**

1. **Hyrule Field to Zora's River:** field exit 3, `ENTR_ZORAS_RIVER_0`, spawn 0 in room 0; no
   intro. Two Octoroks (`En_Okuta` 0xFF00), two blue Tektites (`En_Tite` 0xFFFE, ported). Optional:
   the owl (`En_Owl` 0x01BF, ported: 0x4031 within 360 with Saria's Song and the lullaby, killed
   once `EVENTCHKINF_39`, `z_en_owl.c:204-212, 581-599`), the bean seller (`En_Ms`, a child's
   only, `z_en_ms.c:92-95`) by the soft soil, the boulders over a grotto, gossip stones, cuccos,
   a Skullwalltula in room 1.
2. **The waterfall** (room 1): the song spot `En_Okarina_Tag` 0x07F8 (type 1, switch 0x38, ported)
   asks for Zelda's Lullaby (`z_en_okarina_tag.c:205-228`) and sets switch 0x38 and
   `EVENTCHKINF_39` (`:249-263`); `Bg_Spot03_Taki` 0x0038 sees the switch, starts one-point **4100**,
   fades its vertices out and drops its collision for 400 frames, then closes and clears the
   switch (`z_bg_spot03_taki.c:60, 79-116`): the song is played on every visit. Behind it, exit 3,
   `ENTR_ZORAS_DOMAIN_0`.
3. **The frogs** (`En_Fr`, room 0's log; params 0 the invisible song spot, 1 to 5 the frogs,
   `z_en_fr.c:261-290`): Zelda's Lullaby, Epona's, Saria's, the Sun's Song and the Song of Time give
   a purple rupee the first time each (`EVENTCHKINF_SONGS_FOR_FROGS_*` 0xD1 to 0xD5,
   `save.h:705-712`), the Song of Storms a piece of heart (0xD6), all six their song game (0x40AB,
   `OCARINA_ACTION_FROGS`, a piece of heart, 0xD0). **Not the story's** (BACKLOG #47): this save has
   four of the songs.
4. **Zora's Domain** (spawn 0, room 1; its intro): **King Zora** (`En_Kz` 0x0100, path 1) blocks
   exit 2 (`ENTR_ZORAS_FOUNTAIN_2`): his child text 0x401A sets `player->exchangeItemId =
   EXCH_ITEM_BOTTLE_RUTOS_LETTER`; after the letter 0x401C, with the Sapphire 0x402B
   (`z_en_kz.c:86-97`). Six Zoras (`En_Zo` 0xFFC0 to 0xFFC5, texts; the first talk sets
   `EVENTCHKINF_30`, `z_en_zo.c:379-480`); Ruto beside the king (`En_Ru1` type 5) only after the
   Sapphire (`z_en_ru1.c:2269-2279`); three beehives (`Obj_Comb`), a torch-lit chest (`En_Box`
   0xB7C0, four `Obj_Syokudai`), the fish (`Obj_Mure` 0x5322, type 2, room 1), the waterfalls
   (`Bg_Spot07_Taki` 0, 1).
5. **The diving game** (`En_Diving_Game` at the top of the falls, placed in both rooms, the second
   killed, `z_en_diving_game.c:105-118`): 20 rupees (`:252-256`); the Zora throws `En_Ex_Ruppy`
   (ported) into the pool; five caught before the timer (`Interface_SetTimer`) win the first time
   (ten after, `:160-197`); he gives **`GI_SCALE_SILVER`** and sets `EVENTCHKINF_OBTAINED_SILVER_SCALE`
   0x38 (`:483-497`). Player is held in `PLAYER_CSACTION_8` through it.
6. **Ruto's letter** (Lake Hylia: `Item_Etcetera` 0x0001, `ITEM_ETC_LETTER`, at (-916, -1580,
   4297)): killed once `EVENTCHKINF_31`; offers `GI_BOTTLE_RUTOS_LETTER` within 30 across and 50 up
   or down; taken, it sets `EVENTCHKINF_31` and switch 0x0B (`z_item_etcetera.c:117-151, 175-188`).
   A child's lake is at `WATER_LEVEL_RAISED`, -1313 (`z_bg_spot06_objects.c:43`): the letter lies
   267 below the surface. Player dives to `D_80854784[CUR_UPG_VALUE(UPG_SCALE)]` = 120, 240, 360
   (`z_player.c:10889`, used at `:13636-13640`): with the silver scale he reaches 27 above it,
   within the offer's 50; without it 147 above. **The silver scale is required.** The ways in:
   Hyrule Field (exits 5 and 9, `ENTR_LAKE_HYLIA_0`, its intro), or the Domain's underwater
   tunnel (spot07 exit 4, `ENTR_LAKE_HYLIA_7`, its spawn 231 below the surface: the scale too);
   back by spot06 exit 7, `ENTR_ZORAS_DOMAIN_4`. The lake's other child actors: the scarecrows
   (`En_Kakasi`, `En_Kakasi3`, #47), two Zoras, three Octoroks, three blue Tektites,
   `Bg_Spot06_Objects` (0x02FF **the lake's water surface**: without it a child's lake draws no
   water; 0x001F and 0x011F the Water Temple's gate and lock), Navi's `Elf_Msg2` 0x3B00 by the
   letter, the owl 0x01CA (`OWL_HYLIA_SHORTCUT`, ported: `gLakeHyliaOwlCs` to
   `ENTR_HYRULE_FIELD_9`), `Shot_Sun` 0xFF40, the lab (`En_Mk`: texts for a child).
7. **Showing the letter:** the bottle at King Zora: Player's exchange (text 0x4005, ported),
   `func_80A9CB18` swaps in 0x401B (`z_en_kz.c:291-318`); at its end `EnKz_SetupMweep` (`:426-454`):
   his own sub camera (no script), `PLAYER_CSACTION_8`; `EnKz_Mweep` walks path 1, then
   **`Inventory_ReplaceItem(ITEM_BOTTLE_RUTOS_LETTER, ITEM_BOTTLE_EMPTY)`** and
   **`EVENTCHKINF_GAVE_LETTER_TO_KING_ZORA` 0x33** (`:456-477`). **That's the story's empty
   bottle** (the milk needn't be drunk).
8. **The fish:** no child layer places `En_Fish`: `Obj_Mure` type 2 spawns it (the Domain's 0x5322,
   the fountain's 0x5322 and 0xA322, the lake's 0x2322 and 0x3322); a bottle swing catches it
   (`sBottleCatchInfo`: `ITEM_BOTTLE_FISH`, `z_player.c:14059-14061, 14099-14117`).
9. **Zora's Fountain** (exit 2 past the king, its intro): **Lord Jabu-Jabu** (`En_Jj` -1,
   `JABUJABU_MAIN`) waits for Link within 300 of (-1589, 53, -43) while `play->isPlayerDroppingFish`
   (`Player_IsDroppingFish`: the release action with `PLAYER_IA_BOTTLE_FISH`, `z_player.c:16075-16079`;
   the release spawns `EN_FISH_TYPE_DROPPED` on its frame 76 and empties the bottle, `:14152-14177`);
   100 frames on, `gJabuInhalingCs` (the fish on cue 1, Jabu on cue 2), his body's collision off,
   **`EVENTCHKINF_OPENED_JABU_JABU` 0x3A** (`z_en_jj.c:208-237`); terminator 24 to `ENTR_JABU_JABU_0`
   (the port's table has the row). Later, with 0x3A, he opens his mouth for Link within 300. The
   fountain's other child actors: the bombable wall `Bg_Spot08_Bakudankabe` 0x0003 in front of
   exit 5, Farore's Wind's Great Fairy (`yousei_izumi_yoko`, Phase 20; `Bg_Dy_Yoseizo` is ported
   whole), two gossip stones, pots, butterflies, the fish.
10. **Jabu-Jabu's Belly** (below), **Barinade** (below).
11. **The Sapphire:** Barinade's death spawns `En_Ru1` type 0 at the nearest of three `sWarpPos`
    (`z_boss_va.c:1675-1682`), unconditionally; she spawns `Door_Warp1` **`WARP_BLUE_RUTO`**
    (`z_en_ru1.c:1089-1105`); `DoorWarp1_RutoWarpOut` sets **`EVENTCHKINF_37`**, **`Item_Give(ITEM_ZORA_SAPPHIRE)`**
    and goes to `ENTR_ZORAS_FOUNTAIN_0` with `CS_INDEX_0` (`z_door_warp1.c:558-641`; the port logs
    `RutoWarpIdle`, `door_warp1.rs:351-352`): spot08 layer 4, **`gZorasFountainSapphireCs`** (2,452
    frames: Ruto, `En_Ru1` type 1, on cue 3; `Demo_Effect`'s jewel and light (ported); Navi; texts
    0x4025 with its choice, 0x0082, `CS_TEXT_ZORA_SAPPHIRE` (0x4035 with the Ruby held, ported),
    0x4028); terminator 13 to `ENTR_ZORAS_FOUNTAIN_0` (the port's row). **The phase's exit, part 1.**
12. **The moat** (the exit's part 2): with the three stones a child's Hyrule Field is layer 1; the
    drawbridge (`Bg_Spot00_Hanebasi`, ported) is up and stormy while `EVENTCHKINF_ZELDA_FLED_CASTLE`
    0x80 isn't set; Link at its foot sets 0x80 and `EVENTCHKINF_82` and `CS_INDEX_1`
    (`:220-238`): **`gHyruleFieldZeldaEscapeCs`** (layer 5: `En_Viewer` types 0 to 4, ported; Impa
    throws `Item_Ocarina` type 0 at frame 845, `z_en_viewer.c:378-383`; Ganondorf's cape `En_Ganon_Mant`
    a placeholder, as in the nightmare); terminator 33 back to layer 1. There **`Item_Ocarina`
    0x0003** at (299, -136, 884) in the moat (killed without 0x80 or with `EVENTCHKINF_43`; spawns
    `Elf_Msg2` 0x3800) offers `GI_OCARINA_OF_TIME` within 30 across and 50 up or down, sets 0x43
    and switch 3, and on the box's close starts `gHyruleFieldZeldaSongOfTimeCs`
    (`z_item_ocarina.c:68-76, 179-199`): its terminator 52 to the Temple of Time, `CS_INDEX_7`,
    `gTempleOfTimeSongOfTimeCs` (Zelda `En_Viewer` 0x0202, the jewels, `OCARINA_ACTION_TEACH_TIME`
    and `PLAYBACK_TIME`: the Song of Time given by the message system, `z_message.c:3628`); its
    terminator 53 to `ENTR_HYRULE_FIELD_16`, where `gHyruleFieldGetOoTCs` plays. Ported already:
    the drawbridge's trigger, `En_Viewer`'s escape, terminators 33, 52 and 53, the three-stones
    layer, the cutscenes' ocarina actions, every actor of `tokinoma` layer 11. Missing:
    **`Item_Ocarina`** (213) and `En_Ganon_Mant` (505, an old gap).

### Jabu-Jabu's Belly, Master Quest

16 rooms (0 to 15), 22 transition actors (16 `Door_Shutter`, 6 `En_Holl`), the title card on every
entry (`entrance_table.h:72`), **no small keys, no boss key and no boss door**. Exit 1 back to the
fountain, exit 2 to Barinade (`ENTR_JABU_JABU_BOSS_0`, from room 15).

**The main finding: in MQ neither Ruto nor Big Octo is on the way to Barinade.** The flags give
the way room 5 → door 0 → room 15, and room 5 is reached once the red tentacle is killed with
the boomerang; Barinade spawns his Ruto with no condition.

- **The cows** (`En_Cow`, ported, 14 on the walls) are targets: an `En_Wonder_Item` on each (mode 3,
  `WONDERITEM_INTERACT_SWITCH`, slingshot) drops an item and sets a flag: room 0's west cow 0x15
  opens door 1; room 2's cow 0x13 (inside boulders: bombs) door 4; room 6's 0x20 door 3; others
  chests (nuts, a heart, Bombchus, the Deku Shield) and hookshot posts' risers (`Obj_Hsblock`).
- **The tentacles:** `En_Ba` (red 0x0B00 room 8, gray 0x0A02 room 9, green 0x0C01 room 10) take only
  `DMG_BOOMERANG`, four hits (`z_en_ba.c:62, 133`), and set their flag; each room's `Obj_Roomtimer`
  shares it. `En_Bx`, the electric barriers (indestructible, a barrier quad: 4 damage and a
  knock-back, `z_en_bx.c:150-176`), go with their colour's flag: red 0x0B in rooms 1 (in front of
  door 9 to room 5) and 3, green 0x0C in room 3 (to room 6), gray 0x0A ×2 in room 7.
- **The switches** (`Bg_Bdan_Switch`): blue (held while pressed by anything with
  `ACTOR_FLAG_CAN_PRESS_SWITCHES`: Link, Ruto, a crate `Obj_Kibako`), yellow (stepped on, stays),
  a tall yellow one (hit). No heavy one (the original's Ruto switch): room 7's Ruto (0x0206) never
  appears.
- **`Bg_Bdan_Objects`**: type 0 the octo platform (room 6: waits for Ruto, rises with 3070,
  spawns `En_Bigokuta`, `INFTABLE_RUTO_ABDUCTED`, descends with 3080, the fight, lifts with 3090);
  type 1 room 1's elevator (±471 round y -771 over 512 frames); type 2 room 14's water switch
  (`waterBoxes[7]` +75 for 100 frames); type 3 room 1's falling platform (3100).
- **The spouts** (`En_Siofuki`): lifts; room 3's and 12's rise on yellow switches; room 1's and
  room 14's two drop for good when the boomerang's chest opens (treasure 0x06).
- **The chests:** room 0 the map (a yellow switch under a boulder: bombs); room 1 the compass (the
  tall switch); **room 14 the boomerang** (`En_Box` 0x10C6, `ROOM_CLEAR_BIG`, treasure 0x06: two
  stingers `En_Eiyer`, a Like Like `En_Rr`, a Lizalfos `En_Zf`); room 11 one Deku Stick
  (`ROOM_CLEAR_SMALL`: two caged Like Likes; the save's only stick); the others nuts, a heart,
  Bombchus (10), the Deku Shield.
- **Ruto** (`En_Ru1`): room 2's first meeting (`gRutoFirstMeetingCs`, texts 0x404B, 0x401E; she
  drops through a hole; `INFTABLE_RUTO_MET_FIRST_TIME`); room 3's, carried (0x404C to 0x404E,
  `INFTABLE_RUTO_LET_LINK_CARRY`; thrown she bounces, in water she bobs and sinks, left behind she
  goes); she weighs down room 3's blue 0x3C (door 12, one of two ways to room 14); carried to room
  6 (behind the green barrier), `gRutoFoundSapphireCs`, thrown onto the platform
  `gRutoObtainingSapphireCs`, `INFTABLE_RUTO_HAS_SAPPHIRE`, Big Octo takes her. Player's three
  Ruto checks: a door opened while carrying her (`z_player.c:5382-5384`, missing), her cry on his
  falls (`:9472-9478`, missing), the do-action (ported).
- **The way the flags give** (no Song of Time): room 0 (the west cow, door 1) → room 1 (the
  elevator down, yellow 0x19, door 14) → **room 14 cleared: the boomerang** → room 1, door 2 →
  room 2 (Ruto's meeting; the cow bombed free and shot, door 4) → room 7 → room 11 cleared: the
  stick, lit at the golden torch (lit by the room's clear) → room 7's web (`Bg_Ydan_Sp` 0x1FD6,
  fire) → room 9's gray tentacle (room 7's gray barriers go) → room 7's blue 0x0D under two
  boulders (bombs, a crate on it) → room 8's red tentacle (the red barriers go) → room 1, door 9 →
  room 5: the crystal (`Obj_Switch` 0x1403, flag 0x14) → door 0 → room 15 → exit 2. The slingshot,
  bombs, the boomerang and room 11's stick; Ruto, Big Octo, the map and compass optional.
- **The enemies on that way**, none ported: Octorok (room 0), Shabom (`En_Bubble`: 0, 3, 12), Biri
  (`En_Bili`: 1, 2, 3), Flobbery (`En_Brob`: 1, 2, 4), Stinger (`En_Weiyer`: 1, 13; `En_Eiyer`: 14),
  Like Like (`En_Rr`: 5, 11, 14). Off it: Tailpasaran (`En_Tp`, room 12), Bari (`En_Vali`, room 6),
  Big Octo. Ported: `En_Zf`, `En_Sw` (a Skullwalltula, four Gold Skulltulas), `En_Firefly` 0x8003
  ×3 (room 13, lens-only), `Obj_Hsblock`, `Obj_Timeblock` (room 14's over a Gold Skulltula),
  `Bg_Ydan_Sp`, `Obj_Bombiwa`, `Elf_Msg`, `Elf_Msg2`, `En_Holl`, `Demo_Effect`'s jewel.
- **The draw config** (`Scene_DrawConfigJabuJabu`, `z_scene_table.c:1585-1661`, 77 lines): scrolls
  on 8 and 0x0B, **the view's breathing distortion** every frame (`View_SetDistortion*`, not
  ported), the walls' pulsing matrix on segment 0x0D (y by `sin × 0.8`, room 2 differs), and
  `drawParams[0..1]`'s blue blood on a `FLOOR_TYPE_8` hit (`func_8002F9EC`, `z_actor.c:2120-2128`:
  `En_Arrow`'s seeds, `En_Boom`, Player). No room-RAM or vertex writes.

### Barinade, the warp and the Sapphire

- **`Boss_Va`** (4,011 lines, one file; twice `Boss_Dodongo`): one actor typed by params: the body
  (-1), three supports, three zappers, ten Bari, three stumps, the door's eight pieces; fight
  phases at 3, 9, 15, 18. **The boomerang is the fight:** the supports and the body take only
  `DMG_BOOMERANG` (`z_boss_va.c:229-262`), the stunned body the sword, boomerang and stick
  (`:1161, 1171`). Gohma's pattern for the intro and death (`Cutscene_StartManual`, a sub camera,
  Player's cutscene actions 1, 2, 7, 8), the title card before `EVENTCHKINF_BEGAN_BARINADE_BATTLE`
  0x76 (`:1005-1007, 1047`), `NA_BGM_BOSS`, `Flags_SetClear`, `Item_B_Heart`; **no `Door_Warp1`
  child**: Ruto (above). Re-entered cleared: Ruto, or with `EVENTCHKINF_37` a plain warp, and the
  heart (`:651-663`). New: his own 400-particle effect system (sparks, spark balls, the zappers'
  charge, blood, tumours, gore with a floor raycast, `:3304-3973`), the environment's light and
  fog overrides, `envCtx.screenFillColor`, `EffectSsDtBubble`. His room: six pots of hearts.
- **`Door_Warp1`'s Ruto warp** (`DoorWarp1_RutoWarpIdle`, `_RutoWarpOut`, about 84 lines), and the
  plain warp's later visits (`:536-539`).

### What's already ported and reused

`En_Owl` (the river's and the lake's), `En_Okarina_Tag`, `En_Tite` (blue, on the water), `En_Ishi`,
`En_Kusa`, `Obj_Tsubo`, `En_Wonder_Item`, `En_Wonder_Talk2`, `En_Item00`, `En_A_Obj`, `En_Kanban`,
`En_Gs`, `Door_Ana`, `En_Holl`, `En_Door`, `Door_Shutter`, `En_Box`, `Obj_Switch`, `Obj_Syokudai`,
`Obj_Timeblock`, `Obj_Hsblock`, `Obj_Bombiwa`, `Obj_Bean`, `Obj_Makekinsuta`, `Obj_Mure`, `Obj_Mure2`,
`Obj_Hana`, `Obj_Kibako2`, `En_Wood02`, `En_Niw`, `En_River_Sound`, `Elf_Msg`, `Elf_Msg2`, `En_Sw`,
`En_Firefly`, `En_Zf`, `En_Cow`, `Bg_Ydan_Sp`, `Bg_Haka`, `Bg_Mjin`, `Shot_Sun`, `Demo_Effect`,
`Door_Warp1` (but Ruto's warp), `Item_B_Heart`, `En_Ex_Ruppy`, `En_Butte`, `Eff_Dust`, `En_Elf` (the
bottled fairy), `En_Viewer` and `Bg_Spot00_Hanebasi` (Zelda's escape), `Bg_Dy_Yoseizo`. Player's
lift, carry and throw (ADR 0064: Ruto), the exchange item (Ruto's letter shown, 0x4005), `Item_Give`
for the scale, the letter, the boomerang and the Sapphire, the bottles' contents and half milk;
the HUD timers (the diving game's); the cutscene destinations 13, 24, 33, 52, 53; the entrance
intros; `EffectBlure` (the boomerang's trail); the effects `Kakera`, `KiraKira`, `Hahen`, `EnIce`,
`EnFire`, `DeadDb`, `DeadSound`, `Dust`, `Hitmark`.

### The systems

From Player (`player.rs`, 12,535 lines) and the framework:

| System | C | The port has | Missing |
|---|---|---|---|
| **Swimming** | `z_player.c:6821-6991, 12396-12487, 13440-13785` (~600) | tread, swim, swim to a target, dive, surface, buoyancy, climbing out (`player.rs:8274-8675, 2579-2640`), the in-water item lock, the camera's water settings | **the dive by the scale** (hard-coded 120: `:8630, 11022`); **picking up from the bottom** and surfacing with it (`:13637, 13734-13755`: Ruto's letter); `Player_Action_8084D7C4` (a scene entered in deep water); the cutscene holds in water (`func_80851368`, `func_808513BC`: the diving game's `PLAYER_CSACTION_8`); treading's talk check; `func_8083D6EC`'s water half (ripples, the wading splash, bubbles; ~85) |
| **Currents** | `:11170-11183, 11883-11906` (~40) | the surface's speed read | the conveyor push: Zora's River has 116 water-conveyor polygons, Jabu 106 floor-conveyor ones |
| **Water boxes** | `z_bgcheck.c:4281-4410` | `water_surface` | Zora's Domain's finite box (`sZorasDomainWaterBox`) for actors and Player; `FindWaterBox`, the light index |
| **The underwater view** (#43) | `Camera_UpdateWater`'s half, `Environment_Enable/DisableUnderwaterLights`, `Camera_UpdateDistortion` (84), the view's distortion (`z_view.c:210-305`, ~95) | the settings, `QUAKE_TYPE_5`, the filters | the lights, the quake request, **the distortion** (Jabu's breathing uses the same) |
| **The boomerang** | `Player_InitBoomerangIA`, `func_80835800` to `func_80835C08` (`:3316-3422`, ~120), the hand's model; `En_Boom` (286) | the state flags, the camera modes, the z-target checks, the raise animation | the aim, throw (an `En_Boom` on frame 6 at the focus actor), catch; `En_Boom` (its quad, fetching `En_Item00` and Skulltula tokens, `func_8002F9EC`, through `Bg_Bdan_Objects`, its trail) |
| **Bottles** (#45) | swing and catch (`sBottleCatchInfo`), drink, the fairy's release, a fish's or bug's release, `Player_IsDroppingFish`, the bottle's draw (~290) | the exchange (shown), `Player_UpdateBottleHeld`, `Inventory_UpdateBottleItem`, `Item_Give` | all the actions; **`En_Fish`** (723) and `En_Insect` (816) placeholders (#38) |
| **Ruto carried** | three checks | the generic carry, the do-action | the door while carrying her, her fall cry |
| **The frogs' and scarecrows' ocarina** (#47) | `MSGMODE_FROGS_*` (`z_message.c:3882-3903`, 22); the recording (`general.c:1928-2187`, ~210; `MSGMODE_SCARECROW_*` ~143) | the frogs' mode's start, the recording's constants and state | the frogs' modes' update; the recording |
| **Effects** | `G_Ripple` (137, #74), `G_Splash` (144, #40), `Bubble` (93), `Dt_Bubble` (117), `Lightning` (155) | none of them | all five |
| **Draw configs** | `SDC_ZORAS_RIVER` (24), `_ZORAS_DOMAIN` (27), `_ZORAS_FOUNTAIN` (23), `_LAKE_HYLIA` (22), `_LAKESIDE_LABORATORY` (23), `_JABU_JABU` (77) | none | all six |
| **One-points** | 4100 (68), 4110 (45, the frogs), 3070, 3080, 3090, 3100 (~250) | 5010 (the spouts') | the rest |

### Things to check (for the user, or at a milestone's start)

- **Jabu's climbs** aren't settled (`ootx` dumps no collision): room 1's elevator against its
  ledges and the 1,000 drop, room 14's spouts to the boomerang's ledge (-1073), room 3's spout,
  room 5's posts up to door 0, room 6's lift, room 2's holes. The collision search (GAME-09
  milestone 3's) settles them at the Jabu milestone's start, as far as the way needs.
- **Water boxes aren't dumped either:** the letter's depth (from -1313), the tunnel's, the diving
  game's pool (do its rupees sink below 120?), the moat's depth under `Item_Ocarina` (y -136),
  room 14's `waterBoxes[7]`, room 1's water.
- **Room 11's stick** to room 7's web: about 520 units from the torch; whether a lit stick lasts.
- Room 13's clear: invisible Keese without the Lens (off the way).
- Barinade's title card's size in this ROM (King Dodongo's differed from the XML).
- Where a child's bottle swing reaches a fish (the Domain's group at y -10).
- Jabu's 106 floor-conveyor polygons (a surprise: worth a look).

### Sizes

| Part | Lines of C (about) |
|---|---|
| Water and bottles: Player's dive, bottom pick-up, the holds, the water half, the currents, the bottles; `En_Fish`, `En_Insect`; `G_Ripple`, `G_Splash`, `Bubble`; Zora's Domain's box | 450 + 290 + 723 + 816 + 374 + 45 = 2,700 |
| Zora's River and Zora's Domain: `Bg_Spot03_Taki`, one-point 4100, `En_Okuta`, `En_Zo`, `En_Kz`, `En_Diving_Game`, `Bg_Spot07_Taki`, `Obj_Comb`, `SDC_ZORAS_RIVER`, `_DOMAIN`, the underwater view | 169 + 68 + 738 + 828 + 594 + 595 + 136 + 238 + 51 + 210 = 3,630 |
| The letter, King Zora and Jabu-Jabu's mouth: `Item_Etcetera`, `Bg_Spot06_Objects`, `En_Mk`, `En_Jj`, `Bg_Spot08_Bakudankabe`, `SDC_LAKE_HYLIA`, `_LAKESIDE_LABORATORY`, `_ZORAS_FOUNTAIN` | 261 + 524 + 383 + 492 + 213 + 68 = 1,940 |
| Jabu-Jabu to the boomerang: `SDC_JABU_JABU`, `En_Boom` and Player's boomerang, `Bg_Bdan_Switch`, `Bg_Bdan_Objects`, `En_Siofuki`, `En_Bubble`, `En_Bili`, `En_Brob`, `En_Weiyer`, `En_Eiyer`, `En_Rr`, `Dt_Bubble`, `Lightning`, one-point 3100 | 77 + 406 + 556 + 503 + 328 + 448 + 706 + 342 + 664 + 738 + 942 + 272 + 60 = 6,040 |
| Jabu-Jabu to Barinade: `En_Ru1` (whole, with its scripts), `En_Ba`, `En_Bx`, `Obj_Kibako`, `Obj_Roomtimer`, Player's Ruto checks | 2,766 + 541 + 262 + 299 + 88 + 15 = 3,970 |
| Barinade and the Sapphire: `Boss_Va`, Ruto's warp | 4,011 + 84 = 4,100 |
| The moat: `Item_Ocarina`, `En_Ganon_Mant` | 213 + 505 = 720 |
| The rest of Jabu-Jabu: `En_Bigokuta`, `En_Vali`, `En_Tp`, one-points 3070, 3080, 3090 | 921 + 768 + 729 + 190 = 2,610 |
| The frogs and the scarecrows (#47): `En_Fr`, one-point 4110, `En_Ms`, `En_Kakasi`, `En_Kakasi3`, the frogs' modes, the recording | 1,118 + 45 + 204 + 365 + 457 + 375 = 2,560 |
| Not this phase: `Bg_Spot08_Iceblock`, `Bg_Ice_Shelter` (Phase 16) | (977) |

About 28,000 lines whole (GAME-09's was about 23,000; without the frogs and scarecrows about
25,700). Jabu-Jabu's inside alone is about 12,600, and `Boss_Va` is the largest single overlay yet.

## The proposal

Nine milestones in the story's order. As in GAME-09 the exit can't come first: the Sapphire needs
the scale, the letter, the fish and Jabu-Jabu's way. So the systems first (as bombs were), the
critical path to the Sapphire (milestones 1 to 6, the exit's first part at 6), the moat at once
after (7, the exit's second part), then the breadth (8, 9).

| # | Milestone | What | Run (short debug starts; fights are the hand test's) |
|---|---|---|---|
| 1 | Water and bottles | Player: the dive by the scale, the bottom pick-up and surfacing with an item, `Player_Action_8084D7C4`, the cutscene holds in water, treading's talk check, `func_8083D6EC`'s water half, the currents; the bottles' actions whole (swing and catch, drink, release, `Player_IsDroppingFish`, the draw); `En_Fish`, `En_Insect` (#38, #45); `G_Ripple`, `G_Splash`, `Bubble` (#40, #74); Zora's Domain's water box | Zora's Domain's pool on a debug start with an empty bottle: a fish caught with a swing, the milk drunk; a dive test by scale |
| 2 | Zora's River and Zora's Domain to the silver scale | `Bg_Spot03_Taki` and 4100, `En_Okuta`, `En_Zo`, `En_Kz` (the child's: the adult's ice a placeholder), `En_Diving_Game`, `Bg_Spot07_Taki`, `Obj_Comb`, `SDC_ZORAS_RIVER`, `SDC_ZORAS_DOMAIN`, the underwater view (#43) | The lullaby at the waterfall (the song spot, 4100), through into the Domain (its intro); the diving game won (20 rupees, five caught), the silver scale |
| 3 | Ruto's letter, King Zora and Lord Jabu-Jabu | `Item_Etcetera` (the letter; the fire arrows' and treasure game's types placeholders), `Bg_Spot06_Objects` (the child's water, gate and lock; the adult's parts placeholders), `En_Mk`, `En_Jj`, `Bg_Spot08_Bakudankabe`, `SDC_LAKE_HYLIA`, `_LAKESIDE_LABORATORY`, `_ZORAS_FOUNTAIN` | The letter dived for (the lake over it, with the scale); the letter shown, the king moved, the bottle empty; a fish caught, carried to the fountain, dropped by Jabu-Jabu, `gJabuInhalingCs`, inside (the intro) |
| 4 | Jabu-Jabu to the boomerang | `SDC_JABU_JABU` and the view's distortion, `En_Boom` and Player's boomerang, `Bg_Bdan_Switch`, `Bg_Bdan_Objects` (whole: the octo platform's Big Octo a placeholder until 8), `En_Siofuki`, the enemies on the way (`En_Bubble`, `En_Bili`, `En_Brob`, `En_Weiyer`, `En_Eiyer`, `En_Rr`), `Dt_Bubble`, `Lightning`, one-point 3100; the climbs settled by the collision search | Room 0's cow (door 1); room 1's elevator, yellow 0x19, door 14; room 14 cleared (a debug start): the boomerang's chest |
| 5 | Jabu-Jabu to Barinade | `En_Ru1` whole (Jabu's, the fountain's, the throne room's), Player's Ruto checks, `En_Ba`, `En_Bx`, `Obj_Kibako`, `Obj_Roomtimer` | Ruto's first meeting (room 2) and the cow freed (door 4); room 11's stick lit, room 7's web burned; room 5's crystal (the tentacles' flags a debug start's), door 0, room 15, the fall to Barinade |
| 6 | Barinade and the Zora's Sapphire (the exit's first part) | `Boss_Va` whole, his effects, the environment's overrides; `Door_Warp1`'s Ruto warp | His intro to the fight's start; after his defeat (a debug start) the heart, Ruto, her warp, `gZorasFountainSapphireCs` to the fountain: **the Sapphire** |
| 7 | The Ocarina of Time and the Song of Time (the exit's second part) | `Item_Ocarina` (every type), `En_Ganon_Mant` | From the field with the three stones: the drawbridge, Zelda's escape, the ocarina from the moat, the Temple of Time's Song of Time, back in the field: Phase 12's start |
| 8 | The rest of Jabu-Jabu | `En_Bigokuta`, `En_Vali`, `En_Tp`, one-points 3070, 3080, 3090; rooms 3, 4, 6, 10, 12, 13 whole | Ruto carried onto room 3's blue switch (door 12); carried to room 6 and thrown onto the platform: her two scripts, the platform, Big Octo's arrival (the fight the hand test's) |
| 9 | The frogs and the scarecrows (#47) | `En_Fr` and 4110, the frogs' modes, `En_Ms`, `En_Kakasi`, `En_Kakasi3`, the ocarina's recording | The four songs on the frogs' log (purple rupees); Bonooru records a song |

The scope rule as before: every actor whole, except those a child's init kills (their init's
checks, then a placeholder) and the adult's parts chosen at init.

## The user's calls

2026-10-20: the user took the recommendation on every call below ("our goal is full coverage of
the game's dynamics; don't worry about small details of playability"): the nine milestones in
this order, the moat in (7), the frogs and scarecrows in (9), the adult's parts placeholders chosen
at init with `En_Mk` whole, the view's distortion in, Queen Gohma's heart container fixed at
milestone 1's start, the coverage corrections with milestone 1, Ruto whole in 5 and Big Octo in 8,
the diving game's run from a debug start with the rupees. The milestones are built one after
another; the hand tests come as `game-*.bat` debug starts for what's hard to automate.

The calls as proposed:

1. **The split and its order** as proposed: nine milestones, the systems first, the Sapphire at 6
   and the Ocarina of Time at once after (7), the breadth (8, 9) last. Recommended.
2. **The moat in this phase** (the outline's "then the Ocarina of Time"): `Item_Ocarina` is small
   and the escape is ported; its end, the Ocarina of Time and the Song of Time, is Phase 12's start
   (the Door of Time). Recommended: in, as milestone 7. Or: Phase 12's first milestone.
3. **The frogs and the scarecrows** (#47, 2,560 lines): placed in this phase's scenes (the outline
   names them for Phase 11; Phase 20's row names the scarecrows' songs). Recommended: here, as
   milestone 9 (every actor the phase places). Or: Phase 20, leaving them placeholders.
4. **The adult's parts, placeholders chosen at init** (as `En_Go2`'s): `En_Kz`'s ice
   (`Bg_Ice_Shelter`), `Bg_Spot07_Taki`'s frozen falls, `Bg_Spot06_Objects`' adult branches and its
   ice block (killed for a child), `Item_Etcetera`'s fire arrows (type 7, Phase 16) and the treasure
   game's types (8 to 13, Phase 20). `En_Mk` whole (383: his dive's heart piece and the adult's
   frog trade are small). Recommended.
5. **The view's distortion** (Jabu-Jabu's breathing and the underwater wobble, about 210 lines with
   the underwater lights, #43): recommended in, with milestone 2 (the underwater view) and 4
   (Jabu's draw config), since Jabu's look depends on it. Or: logged, as now.
6. **Queen Gohma's heart container** in the story's presets (GAME-09's exit review): the presets
   from the castle on lack it (four hearts where the game has five here). Recommended: fixed at
   milestone 1's start, the goldens it moves proven and re-recorded on their own. Or: left.
7. **The coverage table's corrections** (above) applied with milestone 1's coverage refresh.
   Recommended.
8. **Ruto and Big Octo off the way** (MQ): Ruto whole in milestone 5 anyway (she's met on the way,
   and her types 0 and 1 make the exit's warp and the Sapphire's cutscene); Big Octo and the
   sapphire's room with the breadth (8). Recommended.
9. **The diving game's 20 rupees:** `fairy-magic` has 16. The story picks rupees up on the way
   (the hand test's); milestone 2's run starts with 20 or more (a debug start).

## Milestone 1: Water and bottles

**The answer:** Player's water and bottles whole for a child, the fish and the bugs, the five
effects water and Jabu-Jabu need, the currents, Zora's Domain's finite water box and the eye under
water. One short scripted run from a debug start, no enemies: **the bottle** (`Route::Bottle`, Zora's
Domain's shallows on `zora-bottle`, the milk on C-Left): the milk drunk twice (half, then the bottle
empty), the empty bottle out and a fish caught with a swing (its text 0x47), and let out again.
The milestone's start also put Queen Gohma's heart container into the story's presets (the user's
call) and applied the coverage table's corrections. Scripts: `scripts\run\test-water.bat`,
`game-water.bat`, `sandbox-water.bat` (menu 152 to 154).

### What was built

- **Player's water** (ADR 0081): the dive by the scale (`D_80854784`: 120, 240, 360), the bottom's
  item picked up in the dive (`Player_ActionHandler_2`) and held up surfacing
  (`Player_Action_8084E1EC`'s `PLAYER_STATE1_10`: frame 10, the turn-around camera's
  `CAM_ITEM_TYPE_8`, then `func_8084DFF4`'s text), the dive's C-button items
  (`Player_ActionHandler_13`), swimming in from an entrance in deep water (`Player_Action_8084D7C4`),
  the cutscene holds in water (`func_80851368`, `func_808513BC`), treading's `Player_IsTalking`,
  `func_8083D6EC` whole (sinking in sand and quicksand, the ripples wading, the splash running
  through water, the bubbles under it) and `func_8083CFA8`'s splash.
- **The currents:** `sConveyorSpeed`, `sConveyorYaw`, `sIsFloorConveyor` read in the collision, the
  water's (`sWaterConveyorSpeeds`) and the conveyor floors' (`sFloorConveyorSpeeds`) push on
  `pushedSpeed`, an exit's walk along its conveyor floor's direction;
  `SurfaceType_GetConveyorDirection` and `_IsFloorConveyor`.
- **Zora's Domain's finite water box:** `CollisionContext::special_water_box` and
  `water_surface_all_hack` (`BgCheck_GetWaterSurfaceAllHack`), set for the Domain; actors, Player
  and the effects ask through it.
- **The bottles:** `Player_Action_SwingBottle` (`sBottleSwingInfo`, `sBottleCatchInfo`: a fairy, a
  fish, blue fire, bugs), `Player_Action_8084EAC0` (drinking: `D_808549FC`, a Poe's
  `Rand_S16Offset`, half the milk), `8084EED8` (a fairy let out), `8084EFC0` (a fish, blue fire or
  bugs let out: `D_80854A28`), their choice in `Player_ActionHandler_13`, the swing from the item's
  button (`func_8083C6B8`), `Player_IsDroppingFish` (`PlayerIface::is_dropping_fish`), the bottle
  drawn in the left hand (`sBottleDLists`, `sBottleColors`, the shield's teal `@bug (game)`).
- **`En_Fish`** whole (by a helper): every type and state, the Jabu-Jabu cutscene fish, the catch
  and the drop; `eng_math::step_to_angle_s` (`Math_StepToAngleS`).
- **`En_Insect`** whole (by a helper): every type, the soil's Gold Skulltula, the water, the catch
  and the drop.
- **`Effect_Ss_G_Ripple`, `G_Splash`, `Bubble`, `Dt_Bubble`, `Lightning`** whole (by a helper), with
  every ported caller wired (actors entering water, `En_Tite`, `En_Ishi`, `En_Kusa`, `Obj_Tsubo`,
  the cuccos, `En_Goroiwa`, the signs, `En_Door`, `En_Part`, the bugs, Player);
  `BgCheck_FindWaterBox`.
- **The underwater view:** `Camera_UpdateWater`'s camera half (`Environment_EnableUnderwaterLights`
  and `_Disable`, `Audio_SetExtraFilter`, the `QUAKE_TYPE_5` sway, the distortion's kind),
  `Camera_UpdateHotRoom`, `Camera_UpdateDistortion`, and `oot_game::view` (`View_SetDistortion*`,
  `View_StepDistortion` on the projection once per `Play_Draw`: `play->viewProjectionMtxF` and the
  renderer's, `DrawLists::distortion`).
- **The presets:** `deku_tree_dead` (Queen Gohma's heart container and her room's flags, which the
  castle's chain and `ocarina-songs` build on); the debug starts `zora-bottle` and `lake-dive`.
- **The run:** `Route::Bottle`, the tasks `Task::UseItem` and `Task::CatchInBottle`, the steps
  `MilkDrunk`, `FishCaught`, `FishLetOut`.
- **Coverage:** the survey's corrections; Phase 11 holds 35 actors (`En_Fish` held there).

### Results

- Tests: `tests/water_bottles.rs` (9: the dive's turn at 120, and at 240 with the silver scale, the
  eye under water, swimming in for 20 frames, milk drunk by halves, a fairy let out, the swing, the
  river's current, Zora's Domain's box, the run), `en_fish.rs` (9), `en_insect.rs` (10), the effects'
  own (18) and `oot_game/tests/effects_water.rs` (6), `oot_game`'s `view` (1); older tests corrected
  (below).
- The run: 667 frames (the milk at 131 and 230, the fish caught at 546, let out at 667). No hit.
- 1,481 tests (one ignored) pass; `cargo test -p layering` passes; no warnings. The pack is still
  format 29 (`out/data26`), re-imported for the new bakes.
- Goldens: against `out/golden_base30`, 181 the same and 92 differ, each proved by run-time
  switches in a proof build (`golden/README.md`): the heart container (69 images, `graveyard`'s
  trace), the currents and the exit floors' conveyor (14 traces, `castle_wave`), the water's
  effects (`meadow`, `spot04_tread`), the scale (`course_swim`'s trace), pairs of them (the castle's
  three, `course_swim`'s sheet, `field_night`'s shot), and `field_night`'s trace by the route alone
  (`step` at frame 1299). The milestone 2 and 3 actors merged since changed no hash. 3 new cases
  (`bottle`, `bottle_drink`, `bottle_catch`): 277 hashes, 210 cases (`out/golden_base31`).

### Decisions

- `Player_UpdateBottleHeld`: Player's half at once, the inventory's as a request
  (`PlayRequest::UpdateBottleItem`), the C's order within the frame (ADR 0081).
- The view's distortion multiplies the projection where the C's does: both the play state's view
  projection (culling, `projectedPos`) and the renderer's.
- Zora's Domain's box lives in the engine's collision context as a general special water box, set
  by the game for that scene only: the engine keeps no scene ids.
- An actor's ripples on entering water are spawned right after its update (the bg check has no
  play state; only the slot order in that frame differs).
- `Route::FieldNight` no longer fights the Stalchildren round the owl's tree: the stream's current
  moved the run and the scripted fight there became a matter of luck (fights are the hand test's).

### Corrections to the survey

- The course's dive (the spikes' debug save) has the silver scale (`Sram_InitDebugSave`'s
  upgrades), so it goes down to the pool's floor; the old test assumed 120.
- An exit's floor is a conveyor: the exit's walk takes its direction (`sConveyorYaw`, a whole 64th
  of a turn); every run's walk out of an exit shows it.
- A diving child's camera stays above the water (`CAM_SET_PIVOT_WATER_SURFACE`); the test puts the
  eye under to check the underwater view.

### Known gaps

- The fishing rod's cast (Phase 20), blue fire's catch (`En_Ice_Hono`, Phase 16), the iron boots.
- Ruto carried (the door and the fall cry: milestone 5); the pick-up from the bottom is run with
  Ruto's letter in milestone 3.

### Fixes found while building

- `tests/en_wf.rs` asserted three hearts (four now); `tests/water.rs`'s dive assumed no scale;
  `tests/camera.rs`'s `CamRoom` (its environment type).

### How to check

```
scripts\run\test-water.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-water.bat
```

### By hand

1. `game-water` (Zora's Domain's shallows, `zora-bottle`): C-Left drinks the milk (Link drinks, the
   hearts fill); again: the last half, the bottle's empty. C-Left takes the empty bottle out (in his
   left hand); walk slowly at a fish in the water and press C-Left again: the swing (in water, the
   scoop's sound); caught, its text, and the bottle shows it (blue). C-Left with the fish: Link pours
   it out, it flaps and swims off.
2. Wading: ripples spread round Link's legs; running through shallow water, splashes; swimming
   under, bubbles rise.
3. `game-water lake` (Lake Hylia, `lake-dive`: the silver scale): A at the surface dives; held, Link
   goes 240 down (the dive count on the A button counts down); without the scale only 120.
4. `game-water river` (Zora's River): swim in the river: the current carries Link downstream.
5. `game-water tunnel` (Lake Hylia from Zora's Domain's tunnel): Link swims in from under water.
6. Anything that looks off: the bottle in hand, the ripples, splashes and bubbles, the fish and the
   bugs (lift a rock in Kakariko or cut grass for bugs: catch them with a bottle too).

## Milestones 2 and 3: Zora's River, Zora's Domain, the letter, King Zora and Jabu-Jabu's mouth

**The answer:** every actor the river, the Domain, the lake, the lab and the fountain place for a
child, ported by helpers and merged (2026-10-20), hand-tested by the user (2026-10-21), and three
short scripted runs from debug starts, no fights:

- **The river** (`Route::ZorasRiver`, on the song ledge in front of the falls on `fairy-magic`):
  Zelda's Lullaby at the song spot, the falls open (one-point 4100), the run-off jump over the
  falls' gap into the tunnel and Zora's Domain with its intro. 633 frames.
- **King Zora** (`Route::KingZora`, 300 in front of the throne on `zora-letter`): Ruto's letter
  shown (0x401B, 0x401C), his mweep aside, the bottle empty. 1,300 frames.
- **Jabu-Jabu** (`Route::JabuFish`, Zora's Fountain on `jabu-fish`): the fish let out by Lord
  Jabu-Jabu, `gJabuInhalingCs`, inside (`SCENE_JABU_JABU`). 843 frames.

The diving game's run was left out by the user's call (2026-10-21): its tests play the whole game
frame by frame, and the user played it. Scripts: `scripts\run\game-zora.bat` (menu 155: the diving
game, `shop`, `king`, `jabu`, `lake`, `lab`, `river`, `boomerang`), `sandbox-zora-runs.bat` (156).

### What was built

- **Zora's River:** `En_Okuta` (every state, its rock), `Bg_Spot03_Taki` and one-point 4100,
  `SDC_ZORAS_RIVER`, `En_Ms` (the bean seller, from milestone 9).
- **Zora's Domain:** `En_Kz` (the child's whole; the adult's a placeholder until `Bg_Ice_Shelter`),
  `En_Zo` (types 0 to 8), `Obj_Comb`, `Bg_Spot07_Taki`, `En_Diving_Game` and `En_Ex_Ruppy`'s
  game, `En_Ossan`'s Zora shopkeeper (his shelves through `EnOssan_SpawnItemsOnShelves`),
  `SDC_ZORAS_DOMAIN`.
- **Lake Hylia and the lab:** `Item_Etcetera` (every type: the letter, the fire arrows, the
  treasure game's), `Bg_Spot06_Objects` (the child's parts; the adult's lock and drained lake
  placeholders until Phase 16), `En_Mk`, `SDC_LAKE_HYLIA`, `SDC_LAKESIDE_LABORATORY`.
- **Zora's Fountain:** `En_Jj` (all three types), `Bg_Spot08_Bakudankabe`, `SDC_ZORAS_FOUNTAIN`.
- **Player:** the dive's jump off a diving board (`Player_Action_80844A44`, `FLOOR_PROPERTY_11`:
  the top of the Domain's falls) into a dive; surfacing with the bottom's item held up
  (`func_8083D12C`: `link_swimer_swim_get`); `PlayerIface::set_yaw` (4100's turn).
- **The presets:** `zoras_domain` (fairy-magic, `EVENTCHKINF_39` and `_A4`) under `zora-bottle`,
  `zora-diving`, `zora-letter`, `jabu-fish` and `boomerang`.
- **The runs:** the three routes, `Who::KingZora`, `Cond::EventChkInf`, their steps.

### Results

- Tests: `tests/zora_runs.rs` (3: the runs, each end checked against the next start's preset),
  `dive_jump.rs`, `zora_shop.rs` (4), `en_ms.rs` (6), and the helpers' (`en_okuta` 12,
  `bg_spot03_taki` 2, `en_kz` 7, `en_zo` 5, `obj_comb` 5, `bg_spot07_taki` 4, `en_diving_game` 7,
  `item_etcetera` 5, `bg_spot06_objects` 4, `en_mk` 3, `en_jj` 5, `bg_spot08_bakudankabe` 3).
- Goldens: all 277 of milestone 1's hashes the same (the presets' new flag changes no run); 7
  new cases: `zoras_river` (the trace, 633 frames), `zoras_river_falls` 320 (one-point 4100, the
  falls parting), `zoras_river_jump` 406 (over the gap), `king_zora` (the trace, 1,300 frames),
  `king_zora_mweep` 1000, `jabu_fish` (the trace, 843 frames), `jabu_fish_inhale` 500. 287
  hashes, 217 cases, the same over two runs (`out/golden_base32`).

### Found by hand (2026-10-21)

- The diving game's dive off the falls was a plain jump (`Player_Action_80844A44` unported);
  surfacing with Ruto's letter didn't hold it up (`func_8083D12C` skipped the carrying flags);
  the bean seller and the Zora shop's keeper and shelves were missing. All fixed.
- The other shops' keepers (the potion shop, the bombchu shop, the bazaar, the Goron shop, the
  Happy Mask shop) are placeholders too: BACKLOG #100.

### Corrections

- Playing Zelda's Lullaby at the falls sets `EVENTCHKINF_39` (`z_en_okarina_tag.c`, type 1; the
  river's owl reads it): the Domain's presets lacked it.
- `ENTR_ZORAS_RIVER_2`'s spawn is in the tunnel behind the falls, across their gap from the song
  ledge: the river run starts on the ledge.
- King Zora's throne faces +z: `ENTR_ZORAS_DOMAIN_1`'s spawn is behind it; the run starts in front.

### By hand

`scripts\run\game-zora.bat` (menu 155), as tested on 2026-10-21; `shop` and the bean seller in
`river` are new since.

## Milestone 4: Jabu-Jabu's Belly to the boomerang

**The answer:** Jabu-Jabu's draw config (the flesh's scrolls, the view's breathing, the walls'
pulse on matrices of their own), its switches, moving parts and spouts, and the six enemies on the
way, all whole; the climbs settled by a search over the floors and the water; four short scripted
runs from debug starts, no fights (the rooms marked cleared):

- **Room 0's cow** (`Route::JabuCow`, the entrance on `jabu-jabu`): the slingshot put on C-Right
  from the item page (the story's save has it on no button), the west cow's target shot (switch
  0x15), door 1 unbarred with its attention camera, through into room 1. 591 frames.
- **Room 1's elevator** (`Route::JabuElevator`, the end of door 1's corridor on `jabu-room-1`):
  the elevator met coming level on its way down by a run-off jump, ridden to the bottom, into the
  pit's water, under the shaft's wall (a dive), out onto the west ledge, the yellow switch 0x19
  (door 2, its attention camera). 728 frames.
- **Room 14's chest** (`Route::JabuBoomerang`, room 14 cleared on `jabu-room-14`, on the chest's
  ledge): the boomerang (`GI_BOOMERANG`, text 0x35, treasure flag 6).
- **Door 14** (`Route::JabuDoor14`, room 1's bottom on `jabu-boomerang`): through door 14 once the
  chest has dropped the spout in front of it.

Scripts: `scripts\run\game-jabu.bat` (menu 157: room 0, `elevator`, `flobbery`, `pit`, `spout`,
`platform`, `room2`, `room14`, `boomerang`, `door14`), `test-jabu.bat` (158),
`sandbox-jabu-runs.bat` (159).

### What was built

- **`SDC_JABU_JABU`** (ADR 0082): the scrolls (segments 8 and 0xB in Jabu-Jabu, 8 in Barinade's
  room), the view's distortion every frame (`View_SetDistortion*`), the pulse quickened by a hit
  on the flesh (`roomCtx.drawParams`, from `func_8002F9EC`: seeds, the boomerang, now the sword),
  and the walls' pulse: segment 0x0D's matrix, which the rooms multiply onto their own before the
  flesh panels' vertices, as a **site matrix** (`eng_gbi::Segment::SiteMatrix`, the pack's
  `DrawList::site_bases`: format 30, `out/data27`) posed by the room draw every frame. Its statics
  carry over a scene change. Added to the importer's draw-config comparison.
- **`Bg_Bdan_Switch`** whole (blue, yellow, heavy, tall, the toggle), **`Bg_Bdan_Objects`** whole
  (Big Octo's platform, its Big Octo a placeholder until milestone 8; the elevator; room 14's
  water box; the falling platform), **`En_Siofuki`** whole (the spouts: the column lifts and pushes
  Link), **one-point 3100** with **`Camera_Special0`**.
- **The enemies** whole, by three helpers: `En_Bubble` (Shabom), `En_Bili` (Biri), `En_Brob`
  (Flobbery), `En_Weiyer` and `En_Eiyer` (Stingers), `En_Rr` (Like Like: the grab, the swallow,
  the theft of the shield or tunic, the drops).
- **Player:** `PlayerIface::player_speed`, `set_player_speed`, `set_pushed` (the spout's writes);
  the sword's `func_8002F9EC` sets `roomCtx.drawParams[0]`.
- **The presets:** `jabu-jabu` (Jabu-Jabu's start: `Route::JabuFish`'s end), `jabu-room-1`,
  `jabu-room-14`, `jabu-boomerang`, each a run's end.
- **The runs:** `Route::JabuCow`, `JabuElevator`, `JabuBoomerang`, `JabuDoor14`; the tasks
  `Task::DiveUnder` (A pressed at the surface and held under it, steering), and the slingshot's
  tasks on whichever C button holds it; `JABU_STARTS` (the hand test's starts, each checked).

### The way, settled

A search over the floors (every floor of each 20-unit cell; walks, climbs to 71, falls; walls by
line tests) and the scene's water boxes settled what the survey left open:

- **Room 1's elevator** (x -60 to 60, z -1650 to -1760; its top -299 to -1242) stops 95 short of
  door 1's corridor (-320) and of door 2's (-340): it's met by a run-off jump as it comes level.
  Its shaft is walled (x about -300) from 62 under the water up. The pit (-1553) is water to -1248
  (`waterBoxes[1]`): the west ledge (-1233, the yellow switch 0x19: door 2) is 160 over the pit's
  ramp, joined to the rest only by the water west of the wall: dive under it. The yellow switch
  opens **door 2** (to room 2), not door 14.
- **Door 14 is shut to Link from room 1 until the boomerang's chest:** room 1's spout 0x15C6 (200
  high, 200 wide) stands across it and pushes him back short of the door (x 257 of the door's
  313). Its switch 0x17 is set by nothing in this ROM's Jabu-Jabu; it drops for good with the
  chest's flag 6 (both room 14's spouts too). So door 14 is room 14's way *out*; the way *in* is
  room 3's door 12 (Ruto on the blue switch 0x3C: milestones 5 and 8).
- **Room 14's chest ledge** (-1073) is joined to nothing a walk, climb or swim reaches: 220 over
  the floor, 135 over the water raised by the yellow switch 0x39 (`waterBoxes[7]`, -1283 to
  -1208). The run starts on it, as the brief planned; how a child gets up there is the hand
  test's (one to look for).
- **Jabu's 106 floor-conveyor polygons** ("Things to check") are room 2's 80 (speed 3, each pointing
  at one of its six holes down to room 3: the floor carries Link into them) and room 7's 26 (speed
  2, drifting its floor to its middle and south door). Milestone 1's currents already push Link
  along them (a test).

### Results

- Tests: `tests/jabu_objects.rs` (10: the yellow, blue and tall switches, the elevator and its
  camera, the falling platform and one-point 3100, room 14's water, Big Octo's platform as far as
  Ruto's part, the lowering and rising spouts, room 2's floor drifting into a hole),
  `tests/jabu_runs.rs` (5: the four runs, each end checked against the next start's preset, and the
  hand test's ten starts standing in their rooms), `oot_game`'s `scene` (2: the config's numbers;
  the rooms' site matrices), the importer's draw-config comparison (Jabu-Jabu and Barinade's room
  added), and the helpers' `en_bubble` (8), `en_bili` (13), `en_brob` (8), `en_weiyer` (5),
  `en_eiyer` (8), `en_rr` (9). `zora_runs`' Jabu-Jabu run now ends as `jabu-jabu`.
- The runs: room 0's cow 591 frames (shot at 181), the elevator 728 (on it at 183, down at 382,
  under the wall at 530), the boomerang's chest 337 (text 0x35), door 14 135.
- 1,632 tests pass, 1 ignored; `cargo test -p layering` passes; no warnings. The pack is format 30
  (`out/data27`).
- Goldens: against `out/golden_base32` 286 of 287 the same; `jabu_fish/shot.png` (its last frame
  inside Jabu-Jabu) differs, proved in `target/game26proof` with run-time switches
  (`out/golden_proof_g10m4.txt`): with Jabu-Jabu's draw config and its new actors both taken out
  it's the base's again; either alone still differs. Its trace is the same. 7 new cases
  (`jabu_cow`, `jabu_cow_shot` 175, `jabu_elevator`, `jabu_elevator_ride` 300, `jabu_boomerang`,
  `jabu_door_14`, `jabu_room2_walls`): 297 hashes, 224 cases (`out/golden_base33`).

### Found by hand (2026-10-21)

- **Link's shadow z-fought on the moving elevator** (the port's): his feet's shadows took their
  floor from a raycast at every drawn frame (60 a second) against the latest game frame's
  collision, while the platform and his feet are drawn blended between game frames: on a platform
  sinking 4 to 5 a frame the shadow fell up to that far under the drawn floor. The floors under the
  feet are now found when each game frame is captured (as the C's `ActorShadow_DrawFeet` finds them
  once a game frame) and blended with the rest (`ShadowRs::feet_floor`); a test. No golden moved.
- **Link stutters slightly up and down on the elevator** (the C's): moving collision is stored as
  whole numbers (`DynaPoly_ExpandSRT`: `BgCheck_Vec3fToVec3s`, `z_bgcheck.c:2963`), so Link stands
  0 to 1 above the drawn platform, a different amount each frame (his steps down 3, 4 or 5 where
  the platform's are 3.6 to 5.3). Left as the game has it (the user's call, 2026-10-21: for now; a draw-only smoothing onto the platform's exact height is the option).

### Corrections to the survey

- Yellow 0x19 opens door 2 (room 2), and door 14 can't be used from room 1 before the chest (above).
- Room 1's pit is water (-1248), the shaft walled: the west ledge is reached under water.
- The story's save has the slingshot on no C button (the bottle, the ocarina, the bombs).
- The story's presets after the diving game (`zora-letter`, `jabu-fish`, `jabu-jabu`) lack the
  silver scale the letter needs (BACKLOG #102): the dive under room 1's wall needs none.

### Known gaps

- Big Octo (`En_Bigokuta`) is a placeholder on its platform until milestone 8; one-points 3070,
  3080, 3090 log until then. Ruto (`En_Ru1`) is milestone 5's.
- The rumble (`Rumble_Request`) isn't modelled.
- `eng_math::rad_to_binang` rounds where the C's `RAD_TO_BINANG` truncates (BACKLOG #101; the spout
  truncates in its own module).

### How to check

```
scripts\run\test-jabu.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-jabu-runs.bat
```

### By hand

`scripts\run\game-jabu.bat` (menu 157):

1. Empty (room 0, `jabu-jabu`): watch the walls breathe and the floor's flesh scroll; the side
   panels' thickness pulses. Fight the Shaboms (they burst on touch: sword them) and the Octorok
   in the water. Put the slingshot on a C button and shoot the west cow's target: door 1 opens with
   a camera on it.
2. `elevator`: run off the corridor's end onto the elevator as it comes down past you, ride it to
   the bottom, swim west, dive under the wall (A at the surface, held), climb out, step on the
   yellow switch (a camera on door 2). The elevator's camera (`CAM_SET_NORMAL2`) while you ride.
3. `flobbery`: the three Flobberies on the west walkway (they rise round you and throw you off;
   the Kokiri Sword stuns them, the boomerang too); the Biris over the shaft (their shock).
4. `pit`: the Stinger in the water (it comes at you swimming, jumps out after you); the tall switch
   on the pit's bottom (the compass's chest).
5. `spout`: walk into the spout across door 14: it lifts you to its top and pushes you back.
6. `platform`: step on the falling platform: the camera under it looking up, the platform drops
   400.
7. `room2`: the Biris, the Flobbery; stand still on the floor: it carries you into a hole (room 3).
8. `room14`: the two Stingers (they come up from the floor), the Like Like (let it swallow you: it
   takes the Deku Shield), the Lizalfos; the yellow switch by the water raises it for 100 frames
   (ticking). Look for a way up to the chest's ledge.
9. `boomerang`: open the chest: the boomerang. `door14` (after it): through door 14.

## Milestone 5: Jabu-Jabu's Belly to Barinade

**Status:** committed 2026-10-21; **the hand test is owed** (the user deferred it:
[HAND-TESTS-OWED.md](HAND-TESTS-OWED.md)), the list below under "By hand".

**The answer:** Ruto (`En_Ru1`) whole, every type, with Player's two Ruto checks; the tentacles,
the electric barriers, the crate and the room timers whole; the silver scale in the story's
presets (BACKLOG #102); the way from the boomerang to Barinade's room settled by the search over
the floors and three short scripted runs from debug starts, no fights:

- **Ruto's first meeting and the cow** (`Route::JabuRuto`, room 2 inside door 2 on
  `jabu-boomerang`, the room marked cleared): Link walks within 265 of Ruto, her cutscene
  (`gRutoFirstMeetingCs`, texts 0x404B, 0x401E) and her drop through a hole; the bombs onto C-Down
  from the item page; a bomb held until its fuse is short and thrown at a run, bursting by the
  cow's three boulders; the cow's target shot (switch 0x13); round room 2's holes to door 4, into
  room 7. Ruto met at frame 804, the boulders at 966, the cow at 1066, through door 4 at 1466.
- **Room 11's stick to room 7's web** (`Route::JabuWeb`, room 11 cleared on `jabu-room-7`, inside
  door 7): the chest's Deku Stick put on C-Left, lit at the golden torch, carried through door 7
  at a run to room 7's web, which burns (switch 0x16). Lit at 629, through the door at 701, the web
  burnt at 810: **the stick lasts** (it burns 210 frames; the way takes about 180 at a run).
- **Room 5's crystal and the fall** (`Route::JabuCrystal`, room 1 in door 9's corridor on
  `jabu-crystal`, the tentacles' flags set by the preset): through door 9 (its red barrier gone),
  down into room 5's pool, the crystal shot with the slingshot (switch 0x14), up the pool's south
  ramp, door 0, room 15's floor: Barinade's room. The crystal at 344, door 0 at 653, Barinade's
  room at 683.

Scripts: `scripts\run\game-jabu.bat` (menu 157) gains `ruto`, `carry`, `stick`, `barriers`,
`gray`, `red`, `green`, `crates`, `crystal`, `room5`, `door0`; `test-jabu.bat` (158) and
`sandbox-jabu-runs.bat` (159) gain the new tests and runs.

### What was built

- **The silver scale** (ADR 0083, the user's call on #102): `zora-letter`, `jabu-fish` and
  `jabu-jabu` (and every preset after) apply the diving game's win: `Item_Give(ITEM_SCALE_SILVER)`
  and `EVENTCHKINF_OBTAINED_SILVER_SCALE` (0x38).
- **`En_Ru1`** whole, by a helper: Barinade's room's (rising through the blue warp she spawns as a
  child), the fountain's (`gZorasFountainSapphireCs`' cues), room 2's (her meeting and drop),
  room 3's (talk: 0x404C to 0x404E; sitting, lifted, carried, thrown, bouncing; in water she bobs,
  sinks and fades; on a blue switch she holds it down), room 6's (`gRutoFoundSapphireCs`,
  `gRutoObtainingSapphireCs`, the Sapphire, Big Octo's platform), the throne room's (0x402C and
  the masks' reactions), beside the door switch, the debug type; her draw (opaque or translucent,
  eyes and mouth, head and chest turned to Link). `Door_Warp1` gains the `rutoWarpState` she reads
  and writes (its Ruto states are milestone 6's).
- **Player:** doors open with Ruto carried (`Player_ActionHandler_1`); her cry on Link's falls
  (`func_80843E14`, `PlayerSfx::AtActor`). `PlayerIface::set_invincibility_timer` (`En_Bx`'s).
- **`En_Ba`** (the tentacles: only the boomerang hurts them, four hits; the swing at Link; the
  blobs on death; their flag), **`En_Bx`** (the barriers: gone with their colour's flag; the quad's
  shock and knock-back), **`Obj_Kibako`** (the crate: lifted, thrown, broken into splinters with
  its drop; on a blue switch it holds it down), **`Obj_Roomtimer`** (the room's clear sets its
  flag; a timed one runs the HUD timer and voids Link out at 0), by a second helper.
- **The presets:** `jabu-room-7` (`Route::JabuRuto`'s end), `jabu-web` (`Route::JabuWeb`'s),
  `jabu-crystal` (a debug start: the way's two tentacles killed, gray 0x0A and red 0x0B, and room
  7's boulders on the blue switch, 0x1C), `jabu-barinade` (`Route::JabuCrystal`'s end: milestone
  6's start).
- **The runs** and `JABU_STARTS`' eleven new starts, each checked (with the switch flags a start
  sets: room 11's timer's).

### The way, settled

- **Room 2:** door 4 and Ruto are reached on foot from door 2. Six holes drop to room 3 (x -80 to
  0 at z -3040 to -3120, 180 to 220 at -3200 to -3240, 0 to 80 at -3280 to -3360, -280 to -240 at
  -3400 to -3440, -100 to -60 at -3520 to -3560), and the floor's conveyors carry Link into them.
  The cow is in a niche in the north wall, 156 over the floor, behind three boulders that break
  only to an explosion within 80 (`Actor_FindNearbyExplosive`). Standing still, A puts a carried
  bomb down; so the bomb is held until its fuse is down to 30 and thrown at a run 130 south of the
  niche, and it bursts against the niche's wall 50 under the boulders. The target is shot from 330
  back (nearer, the seed meets the niche's lip). The "rise" in front of door 4 is an arch over the
  floor.
- **Room 11 to room 7's web:** room 11's golden torch is lit by its timer's flag 0x01 (on the
  room's clear). The stick burns 210 frames, enough for the torch, door 7 and room 7 at a run
  (about 180). Room 7's east side is walled at x 740 north to z -4780; the web stands in the
  alcove beyond. The floor's conveyors (speed 2, towards the room's middle) beat a slow walk: the
  web is reached at a run, the stick's tip crossing its plane.
- **Room 5:** door 9's floor (-320) drops 70 to the pool (its floor -440, the water 20 deep). The
  crystal's sphere is small (20 across, at y -30): shot from the pool, aimed 8 over it for the
  seed's fall. The posts (`Obj_Hsblock`) aren't needed. Link didn't take the step the search found
  back up to the south floor (62 at its lowest, 67 to 71 where he met it); the pool's south rim
  is a ramp from x 1700 (-391) up to x 1460 (-320).
- **Room 15** has no actors: its floor is exit 2's (x 1220 to 1500, z -2119 to -2399), straight
  through door 0.
- A room marked cleared on a debug start doesn't spawn its timer (an enemy until its first update),
  so the start also sets the flag a save that beat the room would have (room 11's 0x01).

### Results

- Tests: the helpers' `en_ru1` (9), `en_ba` (5), `en_bx` (5), `obj_kibako` (5), `obj_roomtimer`
  (4); `jabu_runs` (8: the three new runs, each end checked against its preset, and the 21 starts).
- 1,664 tests pass, 1 ignored; `cargo test -p layering` passes; no warnings. The pack is still
  format 30, re-imported into `out/data27` for the new actors' bakes.
- Goldens: against `out/golden_base33` 294 of 297 the same; `jabu_elevator`'s trace and shot (the
  scale) and `king_zora`'s trace (Ruto's type 5 beside the king, killed at init without
  `EVENTCHKINF_37`) differ, proven with run-time switches in `target/game26proof`
  (`out/golden_proof_g10m5.txt`): with both causes out, all 297 are the base's. 8 new
  (`jabu_ruto`, `jabu_ruto_meeting`, `jabu_web`, `jabu_crystal`, `jabu_tentacle_gray`): 305
  hashes, 229 cases (`out/golden_base34`).
- COVERAGE: Phase 11 28 of 35 (`Boss_Va`, `En_Fr`, `En_Kakasi`, `En_Kakasi3`, `En_Tp`, `En_Vali`,
  `Item_Ocarina` left); 211 ported overall.

### Corrections

- The silver scale is `UPG_SCALE` 1 (`Item_Give` adds `item - ITEM_SCALE_SILVER + 1`), not 2.
- In Master Quest Ruto never sets `INFTABLE_RUTO_PLACED_ON_SWITCH`: she sets it only on a blue
  switch with flag 0x38, and room 3's is 0x3C (she still holds it down: door 12 opens). Room 7's
  Ruto (0x0206) wants that flag, and room 7 doesn't load her object: she never appears.
- Ruto's other places: Zora's Fountain's layer 4 (type 1), Zora's Domain's room 0 in layers 0 and
  1 (type 5, beside King Zora); Barinade's room places none (`Boss_Va` spawns her).

### Known gaps

- `Door_Warp1`'s Ruto states and `Boss_Va` (who spawns Barinade's-room Ruto) are milestone 6's;
  Big Octo is still the platform's placeholder (milestone 8).
- Ruto's first step of her walk to the Sapphire jumps 71 (her root motion; BACKLOG #104,
  milestone 8).
- The tentacles' and barriers' draws haven't been looked at on screen (the hand test).

### How to check

```
scripts\run\test-jabu.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-jabu-runs.bat
```

### By hand

`scripts\run\game-jabu.bat` (menu 157):

1. `ruto`: walk towards Ruto (south-east of you): her meeting, her texts, she walks off and drops
   through a hole. Put the bombs on a C button; hold a bomb under the cow's niche (north wall,
   high up) until it flashes fast, then run and throw it up: the boulders go. Shoot the cow's
   target: door 4 opens.
2. `carry`: room 3 by Ruto: talk to her (she explains, then asks to be carried); lift her with A,
   walk, throw her (she bounces, cries), put her in the water (she bobs, sinks and fades out). Carry her through a door; fall a little with her (her cry). Set her
   on the blue switch: door 12 opens.
3. `stick`: open room 11's chest (a Deku Stick), put it on a C button, light it at the golden
   torch, run through door 7 and into room 7's web on the east side: it burns.
4. `barriers`: room 7's gray barriers: touch one (a shock, knock-back).
5. `gray`, `red`, `green`: each tentacle with the boomerang (put it on a C button): sword hits do
   nothing, four boomerang hits kill it, its room clears, its barriers go. Let it swing at you.
6. `crates`: lift a crate, throw it (it breaks); carry the other onto the blue switch between the
   blown boulders: door 6 opens.
7. `crystal`, `room5`: through door 9 into room 5, shoot the crystal over the pool (door 0's
   camera), up the south ramp. `door0`: through door 0, the fall into Barinade's room.
