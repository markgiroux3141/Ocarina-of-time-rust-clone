# Roadmap: the next phases

[ARCHITECTURE-PLAN.md](ARCHITECTURE-PLAN.md) §5 defines Phases 0 to 3 and lists what comes after only roughly. This file breaks the next phases into milestones, so each session can start on a known piece of work. Each milestone is sized like the GAME-02 ones: one session, one commit, with tests and a results section in its phase doc.

The working rules don't change:
- no game data in the repo;
- the engine never depends on game code;
- the runtime reads only the pack;
- ports go function by function, with every constant cited and faithful bugs marked `@bug (game)`;
- test expectations come from the C;
- decisions are recorded as ADRs.

Reorder freely. The dependencies are noted, and nothing here is a commitment.

## The aim

Decided 2026-10-13 ([ADR 0062](adr/0062-the-aim-master-quest-and-the-game-editor.md)):
- **Replicate the whole game, and above all capture every game object and dynamic, so that we can
  make our own version of the game with the editor.** The faithful port (the C's functions,
  constants and bugs; tests from the C) is how the dynamics are captured exactly. What it serves
  is a game whose every object and behaviour the editor can place, configure and play.
- **Master Quest throughout:** every dungeon is the debug ROM's Master Quest dungeon (ADR 0028's
  call for the Deku Tree, now for all of them).
- **The game editor:** the overworld editor is the first mode of an editor for every part of the
  game ("The game editor track", below).

Where the plan goes from here: [The whole game](#the-whole-game-outline-2026-10-13) outlines
Phases 9 to 20; [COVERAGE.md](COVERAGE.md) (generated) lists every scene's actors by phase against
what's ported; [How a phase starts and how a milestone is done](#how-a-phase-starts-and-how-a-milestone-is-done)
is the checklist every milestone follows.

## Where things stand (2026-10-13)

**Done:** Phases 0 to 4 (GAME-01, GAME-02 and GAME-03). Kokiri Forest plays with:
- collisions, props, climbing, Z-targeting, doors and the prerendered interiors;
- talking, the message box, drops and a minimal HUD;
- the Deku Tree's mouth, opened by a save flag.

GAME-03, Phase 4, is done too:
- the inventory, `Item_Give`, the get-item flow, the chests, and Link's equipment from the save;
- the crawl and the crawlspace's camera, the scene paths, the training area's boulder and wonder
  items, and Link's knockdown;
- Mido (`En_Md`), the Kokiri shop (`En_Ossan`, `En_GirlA`, `En_Tana`), and the pause menu's
  equipping stand-in in the play frame (`KaleidoSetup_Update`);
- the cutscene system (`z_demo.c`): the scripts from the pack, sub cameras and `Camera_Demo1`,
  the entrance triggers, Player's cutscene modes; the Deku Tree's talk opens his mouth, and his
  scene's intro plays;
- Navi (`En_Elf`) and the Kokiri children's fairies, the target context's `naviRefPos`, C-Up and
  her texts (`z_elf_message.c`), the actors' point lights; every scene's cutscene layers, and a
  new file that starts, as the file select starts it, with the game's opening.

Five headless scripted runs (`oot_actors --test playthrough`). Four start in Link's bed:
- into the Deku Tree, on a flag preset;
- on a new save to the Kokiri Sword's chest, opened;
- on a new save to the Deku Shield bought, both worn, and past Mido;
- on a new save past Mido, through the Deku Tree's talk and into the Deku Tree, with no preset.

The fifth is Phase 4's exit: from the file select's new file, its first frame, through the
opening, C-Up to Navi, and the same way into the Deku Tree.

Phase 5, GAME-04, is done too (see [GAME-04](GAME-04-audio.md)):
- the audio library ported whole (`eng_audio`): the heap and loads, the sequence player, the
  notes and envelopes, the synthesis, and the RSP's audio microcode, offline as the console
  renders and through the output device in the window;
- the game's music: each scene's sound settings, the sequence commands, the scene's music, the
  nature ambience, the fanfares, the fades between scenes;
- the sound effects: the engine whole, positioned by the actors' `projectedPos` and the sound
  sources, and the calls of Player, the message box, the HUD, the targeting, the camera, the
  collision check and every ported actor;
- the phase's exit: scripted runs' sound effect requests checked against the C's calls frame by
  frame, and an audio golden.

Now the cutscene phase (GAME-04b, [GAME-04b-cutscenes.md](GAME-04b-cutscenes.md)), between
Phase 5 and Phase 6: all seven milestones are done (the one-point cutscenes: the crawlspace's
exit, the attention cameras, the falling chest's shot; the jump, climb and hang cameras; the
cutscenes' music and sounds, and Player's cutscene modes from the C's tables; the environment
across frames: the lights' override and blend, the rain, the lightning; the title cards; the
opening's nightmare with its riders on their horses (skin skeletons, ADR 0030) and the
drawbridge; the cutscenes checked against the C frame by frame), and the user has played the
opening and the Deku Tree's talk by hand.

Now Phase 6, the Deku Tree, on Master Quest (GAME-05, [GAME-05-deku-tree.md](GAME-05-deku-tree.md)).
Milestone 1, the decomp upgrade, is done ([ADR 0031](adr/0031-decomp-main.md)): the repo cites
zeldaret/oot main at `52a510f`, migrated from `2f4c25d` through a generated name map
(`docs/name-map`), the importer reads main's layout, and the pack is format 16. Every test passes
and every render is the same bytes; the traces are the old ones' bytes once renamed.
Milestone 2, damage and health, is done ([ADR 0032](adr/0032-damage-death-and-the-game-over-stand-in.md)):
every hit kind, the red flash, death and the game over (its menu a stand-in, undrawn), a bottled
fairy's revival, the damage tables, and `En_Dekubaba` pulled forward; pack format 17.
Milestone 3a, combat basics, is done ([ADR 0033](adr/0033-effects.md),
[ADR 0034](adr/0034-guard-battle-camera-and-the-first-enemies.md)): the guard with the shield
(blocking, a Deku nut bounced back), `Camera_Battle1`, the effects (`EffectSs` and
`z_effect.c`'s), the Keese and the withered Deku Baba, drops on death; pack format 18.
Milestone 3b, the rest of the Deku Tree's enemies, is done
([ADR 0037](adr/0037-the-deku-trees-other-enemies.md)): the Deku Scrubs (with the salesman a
Business Scrub becomes), the Skulltula, the Skullwalltulas and Gold Skulltulas (with the token,
the save's Gold Skulltula flags), Gohma's eggs and larvae (their boss side waiting for
`Boss_Goma`), four more effect overlays; pack format 19.
Milestone 4a, the dungeon's doors, switches, torches and webs, is done
([ADR 0038](adr/0038-sliding-doors-and-room-travel.md), [ADR 0039](adr/0039-quakes.md),
[ADR 0040](adr/0040-switches-torches-webs-and-the-map-data.md)): the sliding doors with Player's
side and small keys, the quakes, the switches, torches, webs and Navi's hint spots, the map and
compass as data, a debug start per room; pack format 20.
Milestone 4b, the Deku Stick and the props, is done
([ADR 0041](adr/0041-the-deku-stick-and-the-item-buttons.md),
[ADR 0042](adr/0042-the-deku-trees-props-and-the-fragments.md)): the Deku Stick from C-Left
(Player's item buttons and item change whole), lit at the torches, burning, breaking, the webs it
burns; room 5's log and floating block with its water, room 10's rising platforms, room 2's lift
and ladder, the crates, and the fragments (`Effect_Ss_Kakera`) the bushes and rocks now spawn too;
pack format 21. Milestone 4c, pushing and Master Quest's extras, is done
([ADR 0043](adr/0043-push-and-pull.md), [ADR 0044](adr/0044-master-quests-extras-and-room-travel.md)):
Player's push and pull, room 3's push block into the pit, the gravestones, the Song of Time's
blocks and the rocks (their song and explosions injected), and a test walking every connection
milestone 4 opened; pack format 22. Milestone 5, items in use, is split in three (decided
2026-10-07): 5a, the Fairy Slingshot and Deku nuts, is done
([ADR 0045](adr/0045-the-fairy-slingshot-first-person-and-deku-nuts.md),
[ADR 0046](adr/0046-en-arrow-the-nuts-stun-and-the-flash.md)): Player's slingshot and first person
(C-Up's look and the aim, `Camera_Subj3`), the nut's throw, `En_Arrow`, `En_M_Fire1`,
`Effect_Ss_Stone1` and the screen's flash, the eye switches and room 2's ladder hit for real, the
room travel test round rooms 1 to 8; pack format 23. 5b, the pause menu, is split in two (decided
2026-10-07): 5b-1, its frame and the item page, is done
([ADR 0047](adr/0047-the-pause-menu.md)): `KaleidoSetup`, the opening and closing at 30 frames a
second, the four pages' box and turns, the cursor, the name and info panels, the item page whole
with the equip's flight to the C button, the other pages' backgrounds, the scene behind kept,
the HUD's START and B labels; Start's stand-in down to its equipment half, at the menu's resume;
pack format 24. 5b-2, the dungeon map page and the game over drawn, is done
([ADR 0048](adr/0048-the-pause-map-and-the-game-over.md)): `KaleidoScope_DrawDungeonMap` whole
with its loads and `z_lmap_mark.c`'s marks, the room maps drawn from the game's own texels through
their run-time palette (a per-draw image in the engine), "GAME OVER" and the game over's prompt
page; pack format 25. 5c, saving, is done ([ADR 0049](adr/0049-saving.md)): `z_sram.c` whole for
this ROM on an SRAM image the play state carries (the save file `out\saves\<ROM SHA-1>.sra`,
bound by the apps only), every field of `Save` laid out in the C's bytes, the pause menu's save
prompt and its page, the game over's `Sram_WriteSave`, `--file N` (and `--new-file`) for the file
select's load, the debug starts on an SRAM in memory, F5 the console's reset, `ootx sram` for the
file select's erase and copy; pack format 26. Milestone 5 is done. Milestone 6, Gohma, is split in three (decided
2026-10-09): 6a, Queen Gohma, is done ([ADR 0050](adr/0050-queen-gohma.md),
[ADR 0051](adr/0051-textures-replaced-by-source-and-the-object-ram.md)): `Boss_Goma` whole with
her intro and death (her own sub camera), the boss title card, her draw with her decay (textures
replaced by their source address, the object RAM the game writes), `Item_B_Heart`, the jump
slash pulled forward; pack format 27. 6b, the blue warp, is done
([ADR 0052](adr/0052-the-blue-warp.md)): `Door_Warp1` for this ROM's Deku Tree, its portal on two
matrices the draw computes, one-point 9703, Player's arrival by blue warp, Kokiri Forest's emerald
cutscene to its terminator. 6c, the run through the Deku Tree, is done: room 9 into Gohma's
room as a short golden, and the travel test carried on through her fight and the warp, the whole
Deku Tree from one `Play_Init`. Milestone 6 is done, and with it Phase 6 (GAME-05).

Now Phase 7, the road out of the forest (GAME-06,
[GAME-06-out-of-the-forest.md](GAME-06-out-of-the-forest.md)): surveyed and proposed
(2026-10-09); the split decided the same day (Kakariko and Castle Town moved to Phase 8).
Milestone 1a, the emerald and Farore's light, is done
([ADR 0053](adr/0053-demo-effect-curve-skeletons-and-vertices-by-source.md)): `Demo_Effect`
whole, curve skeletons (`z_fcurve_data_skelanime.c`), bakes that know their vertices' sources so
a draw rebuilds what the game writes into them, `--cutscene` for a debug start; Kokiri Forest's
cutscene layers 4 and 6 play whole (the emerald over Link, the Deku Tree's death); pack format 28.
Milestone 1b, the creation, is done ([ADR 0054](adr/0054-the-128-skies-and-demo-kankyo.md)):
`Demo_Kankyo` whole, `Bg_Spot09_Obj`, `Bg_Spot16_Doughnut`, two scene draw configs, the outdoor
skies (`SKYBOX_DRAW_128`, loaded as `Environment_UpdateSkybox` loads them) and a hidden room; the
whole chain plays from the blue warp to `ENTR_KOKIRI_FOREST_11`, and closes BACKLOG #23.
Milestone 2, leaving the forest, is done ([ADR 0055](adr/0055-leaving-the-forest.md)): Mido's
talk, Saria's goodbye on the bridge and the Fairy Ocarina (`Demo_Sa`'s bridge), the soft soil,
Hyrule Field's intro, and the owl whole with his one-point cutscene. Milestone 3, the clock, is
done ([ADR 0056](adr/0056-the-clock.md)): time passes from the rooms' settings, on the save, with
the sun and the moon, the skybox filters, the lens flare (its occlusion read back by a depth
probe), the music by the time and the drawbridge's night. Milestone 4, Hyrule Field, is done
([ADR 0057](adr/0057-hyrule-fields-actors.md)): its trees, bush and rock circles, signs and
grottos' holes, its Peahats by day and its Stalchildren by night (breaking into their bones), the
enemy music; its two exit runs reach Castle Town's entrance by day and Kakariko at 20:00. Phase 7
is done.

Now Phase 8, Castle Town, Hyrule Castle and Kakariko (GAME-07,
[GAME-07-castle-town-and-kakariko.md](GAME-07-castle-town-and-kakariko.md)): surveyed, proposed
and decided (2026-10-11): six milestones in the story's order, the ocarina before Zelda, the
culling volume first. Milestone 1, Castle Town, is done
([ADR 0058](adr/0058-the-culling-volume.md), [ADR 0059](adr/0059-castle-towns-actors.md)): the
culling volume; the market's people (the townsfolk, Malon, the haggling pair, the dancing couple,
the cucco girl and her cucco, the dogs, the guards, the butterflies), the gossip stones, the
Temple's altar, windows and warp pad; `Camera_Fixed1`; the game's 4:3 frame shown at 16:9,
black beside it (BACKLOG #1). Its exit run goes from the entrance through the market to the castle's road.
Milestone 2, Hyrule Castle's grounds, is done ([ADR 0060](adr/0060-hyrule-castles-grounds.md)):
Talon asleep by the crates and woken by the Cucco (Player's exchange items), his run home, the
milk crates, the sentries and their throw-out through the circle wipe (`TransitionCircle`), the
gate guards and gates (Lon Lon's fence, Kakariko's), the pots, the torches, the moat's water
camera (`Camera_Jump3`). Its exit run goes from the castle's road past Talon and the crates
through the drain into the guards' courtyard; a second run is caught and thrown out.
Milestone 3, the ocarina, is done ([ADR 0061](adr/0061-the-ocarina.md)): Player's ocarina, the
message box's staff, notes and modes (free play, check, teach, playback), the ocarina's audio, the
songs' effects (`Oceff_Wipe`, `_Wipe2`, `_Wipe3`, `_Wipe4`, `_Spot`, `_Storm`, `En_Okarina_Effect`),
the Sun's Song's time, the fairy spots (`Shot_Sun`), the song spots (`En_Okarina_Tag`), and the
cutscenes' ocarina texts. Its exit run plays Saria's Song (Navi calls, Saria talks), the Sun's Song
(night to dawn) and the Song of Storms at the fairy spot (a fairy) in Hyrule Field.
Milestone 4, the guards' courtyard and Zelda, is done ([ADR 0063](adr/0063-the-courtyards-zelda-and-impa.md)):
the side-scrolling courtyard and its patrols, Zelda's meeting, the legend and her letter, Impa's
lullaby and her goodbye in Hyrule Field.
Milestone 5, Kakariko, is done ([ADR 0064](adr/0064-kakariko-lifting-and-the-village.md)):
Player's lift, carry and throw with the cucco's glide; Anju and her cuccos, the attack cuccos, the
carpenters, their boss and the son, the man on the roof, the windmill, the well and the gallery;
the cow, the windmill man (the Song of Storms drains the well), the House of Skulltula's family;
the village's, the grottos' and the guard house's draw configs (the windows light at dusk); every
actor destroyed on a scene change. Its exit run carries three cuccos into Anju's pen (one glided
off a roof) and opens the gate to Death Mountain with Zelda's letter.
Milestone 6, the graveyard, is done ([ADR 0065](adr/0065-the-graveyard-the-royal-tomb-and-the-suns-song.md)):
Player's grab and auto lock-on; the graveyard kid, Dampé and his dig, the royal tomb's gravestone
and its opening (the blast, `EffectSsBomb2`), the thunderstorm by the tomb, the ReDeads, the
graves' walls, the soft soil's Gold Skulltula; the graves' and the tomb's draw configs. Its exit
run digs with Dampé at 18:30, opens the royal tomb with Zelda's Lullaby, clears its Keese, passes
its ReDeads, learns the Sun's Song at its wall and stuns the ReDeads with it.
**Phase 8 is done**, but for the Temple of Time's room 0 (BACKLOG #37: `Bg_Toki_Swd`, `Demo_6K`,
`En_Xc`), which waits for the Door of Time. Phase 9 starts with its survey.

Phase 9 (GAME-08, [GAME-08-woods-meadow-and-ranch.md](GAME-08-woods-meadow-and-ranch.md)) is done:
the meadow and Saria's Song, the Lost Woods' Skull Kids and scrubs, Lon Lon Ranch and Talon's house.

Now Phase 10, Death Mountain, Goron City and Dodongo's Cavern (GAME-09,
[GAME-09-death-mountain-and-dodongos-cavern.md](GAME-09-death-mountain-and-dodongos-cavern.md)):
surveyed, proposed and decided (2026-10-15): six milestones, the critical path to the Goron's Ruby
first. Milestone 1, bombs, is done ([ADR 0072](adr/0072-bombs-and-bomb-flowers.md)): `En_Bom`,
`En_Bombf`, the fuse's sparks, Player's explosive in hand and the Bomb Flower's hand-over;
BACKLOG #33 closed. Its run puts a Bomb Flower down at Dodongo's Cavern's entrance wall (the
cavern's opening) and a bomb from the bag by the other flower. Milestone 2 (the trail and Goron
City to the bracelet, [ADR 0073](adr/0073-death-mountain-trail-goron-city-and-the-bracelet.md)) and
milestone 3 (the cavern's route, [ADR 0074](adr/0074-dodongos-cavern-route-the-skull-and-effect-blure.md):
the loop to the skull's bridge, the skull, `EffectBlure`) are done, and milestone 4 (King Dodongo
and the Goron's Ruby, [ADR 0075](adr/0075-king-dodongo-the-room-texture-the-cameras-up-and-the-ruby.md))
is done: **the phase's exit is met**. Milestone 6 (the summit, the Great Fairy and magic,
[ADR 0076](adr/0076-the-magic-meter-the-great-fairy-the-spin-attack-and-the-return-entrances.md):
the magic meter, the Great Fairy, the spin attack, the return entrances; after the hand test the
actors' shadows the C's way) was taken before 5 and is done, and milestone 5 (the rest of the
cavern, [ADR 0080](adr/0080-the-rest-of-dodongos-cavern.md): the Dodongos, the Beamos, the silver
rupees, the falling stairs) is done: every actor Phase 10's scenes place is ported. **Phase 10 is
done** with its exit review (2026-10-20): Phase 11 starts from the `fairy-magic` preset.

Phase 11 (GAME-10, [GAME-10-zoras-waters-and-jabu-jabu.md](GAME-10-zoras-waters-and-jabu-jabu.md)):
surveyed and proposed (2026-10-20), nine milestones, every call the recommended one. Milestone 1
(water and bottles, [ADR 0081](adr/0081-water-bottles-and-the-underwater-view.md)) is done;
milestones 2 and 3 (the river, the Domain, the lake, the fountain: King Zora and Jabu-Jabu's mouth)
are done (2026-10-21); milestone 4 (Jabu-Jabu to the boomerang, [ADR 0082](adr/0082-jabu-jabus-belly-to-the-boomerang.md))
is done; milestone 5 (to Barinade: Ruto whole, the tentacles, barriers, crates and room timers,
[ADR 0083](adr/0083-jabu-jabus-belly-to-barinade.md)) is done; milestone 6 (Barinade and the
Sapphire, [ADR 0084](adr/0084-barinade-rutos-warp-and-the-zoras-sapphire.md)) is done;
milestone 7 (the Ocarina of Time and the Song of Time, [ADR 0085](adr/0085-the-ocarina-of-time-and-the-song-of-time.md))
is done; milestone 8 (the rest of Jabu-Jabu: Big Octo, the Bari, the Tailpasarans, Ruto's way to
the Sapphire, [ADR 0086](adr/0086-the-rest-of-jabu-jabus-belly.md)) is done (all four hand-tested
2026-10-22); milestone 9 (the frogs and the scarecrows: the ocarina's recording and the box's frog
and scarecrow modes, `En_Fr`, `En_Kakasi`, `En_Kakasi3`, `En_Kakasi2`, [ADR 0087](adr/0087-the-frogs-and-the-scarecrows.md))
is done (2026-10-23, its hand test deferred: HAND-TESTS-OWED.md). **Phase 11 is done** with its
exit review (2026-10-23, GAME-10's last section): Phase 12 starts from `ocarina-of-time`.

Phase 12 (GAME-11, [GAME-11-the-temple-of-time-and-adult-link.md](GAME-11-the-temple-of-time-and-adult-link.md)):
surveyed and proposed (2026-10-23), three milestones, every call the recommended one. Milestone 1
(adult Link and the time machine: the age switch, the pedestal, Player's time travel,
[ADR 0088](adr/0088-the-age-switch.md)), milestone 2 (the Door of Time to the Chamber of the Sages
and out, the exit, [ADR 0089](adr/0089-the-door-of-time-to-the-chamber-and-out.md)) and milestone
3 (Sheik's Prelude, time travel both ways and the warp,
[ADR 0090](adr/0090-the-prelude-and-time-travel-both-ways.md)) are done, all hand-tested.
**Phase 12 is done** with its exit review (2026-10-24, GAME-11's last section): Phase 13 starts
from `adult-awake`.

Phase 13 (GAME-12, [GAME-12-epona-and-the-adult-field.md](GAME-12-epona-and-the-adult-field.md)):
surveyed and proposed (2026-10-24), four milestones, every call the recommended one. Milestone 1
(riding Epona, [ADR 0091](adr/0091-riding-epona.md)), milestone 2 (Ingo, the rental and the
races, [ADR 0092](adr/0092-ingo-the-rental-and-the-races.md)), milestone 3 (Epona won and out,
the exit, [ADR 0093](adr/0093-epona-won-and-out.md)) and milestone 4 (the ranch after Epona and
the field's Poes, [ADR 0095](adr/0095-the-ranch-after-epona-and-the-fields-poes.md)) are done, all
hand-tested. **Phase 13 is done** with its exit review (2026-10-26, GAME-12's last section): Phase
14 starts from `epona-won`.

**Priorities (2026-10-01):** the cutscenes are finished properly now, as their own phase, so later
work doesn't have to think about them (the user decided against skipping them). This replaces
the 2026-09-30 deferral: the scripted runs play the cutscenes.

**Shortcuts the next phases have to undo:**
- ~~A new save gives child Link the Kokiri Sword and a shield on B.~~ Undone in GAME-03 milestone 1: a new save is `Sram_InitNewSave`'s, and the presets own and wear them (ADR 0019).
- ~~The inventory isn't kept.~~ Undone in GAME-03 milestone 1.
- The pause menu's frame, item page and dungeon map page are ported (ADR 0047, 0048); the equipment and quest pages and the world map show only their backgrounds (their contents logged), and the equipment page's equipping is a stand-in run as the menu closes: every owned piece of a type with nothing worn goes on (ADR 0019, 0047). The debug inventory editor logs. The save prompt saves (5c, ADR 0049), but the title screen and the file select aren't ported: `--file N` stands in for the load (`--new-file` for a new file, `ootx sram` for erase and copy), a debug start saves to an SRAM in memory, and F5 stands in for the console's reset. The game over's screens are drawn (5b-2); "Continue? No" respawns instead of going to the title screen (ADR 0032).
- Unported actors are placeholders. Among them: Saria, the opening's nightmare (`En_Viewer`, the drawbridge), and every shopkeeper but the Kokiri one. (Mido and the Kokiri shopkeeper: ported in GAME-03 milestone 3. Navi and the other fairies: GAME-03 milestone 5.)
- ~~There are no cutscenes: the importer skips all 73.~~ Undone in GAME-03 milestone 4 (ADR 0022): the 73 scene scripts and the 27 overlay ones are in the pack, and the Deku Tree's talk opens his mouth on a new save. The `deku-tree-open` preset stays for the shortcuts and GAME-02's run.
- ~~The pack holds scene layers 0 to 3 only: no cutscene layers, so a new file doesn't start with Navi's wake-up.~~ Undone in GAME-03 milestone 5 (ADR 0023): every scene's cutscene layers are in the pack, and a new file plays the opening. The routes from Link's bed keep their start (`cutsceneIndex` 0).
- No effects (`EffectSs`): Navi's sparkles, and the effects' own `Rand` calls, are missing.
- ~~No audio at all.~~ Undone in GAME-04 (ADRs 0024 to 0027): the library, the game's music and the
  sound effects; the cutscenes' music and sounds in GAME-04b milestone 3.

## Phase 3, the end: GAME-02 milestone 4 (done)

Done as planned, with one correction: the open mouth is `EVENTCHKINF_05`, not `EVENTCHKINF_07` (see [GAME-02](GAME-02-kokiri-forest.md) milestone 4 and ADR 0018). The plan as it was:

**Goal:** close the Kokiri Forest slice with its exit test from the plan: a headless scripted playthrough as a regression test.

**What the C needs.** The Deku Tree's mouth is cutscene-driven (`z_bg_treemouth.c`):
- Talking to the tree starts a cutscene (`gDekuTreeMeetingCs`, setting `EVENTCHKINF_0C` and `EVENTCHKINF_05`).
- The open mouth is drawn from `EVENTCHKINF_07` (env alpha 2150 instead of 500). *(Corrected in the milestone: `EVENTCHKINF_07` is the tree dead, set by `Door_Warp1` after Gohma, and only picks the colour. `EVENTCHKINF_05`, set by `func_808BC9EC` when Link says yes, holds the mouth open.)*

Without a cutscene system, this milestone drives the mouth by save flags. The talk cutscene waits for Phase 4.

- **`Bg_Treemouth`:**
  - `BgTreemouth_Init`: DynaPoly `gDekuTreeMouthCol`, the layer and age cases;
  - the draw, with the env alpha as a dynamic colour;
  - the flag-driven states: `func_808BC8B8` without its cutscene triggers, logged as not ported.
- **A debug save preset** for the sandbox and the tests:
  - `EVENTCHKINF_07` set, so the mouth is open;
  - optionally, the sword and shield flags, so Mido's placeholder story holds.
- **The playthrough** (`oot_actors` test and a sandbox script), with assertions at each step:
  1. Link's house;
  2. out the door;
  3. read the sign;
  4. talk to a Kokiri child;
  5. cut a bush and collect its drop;
  6. walk to the tree;
  7. into the mouth: the exit to `ENTR_DEKU_TREE_0`, ending in the Deku Tree scene.

  The same inputs also write a JSON trace, added to the goldens.
- **The interactive checks** listed at the end of GAME-02, against Project64, before recording the playthrough.

**Exit:** the playthrough passes headless; GAME-02 is marked complete; the architecture plan's Phase 3 is marked done.

## Phase 4: the road to the Deku Tree (GAME-03, done)

**Goal:** a new save plays Kokiri Forest the game's way, up to entering the Deku Tree:
- wake up;
- get the Kokiri Sword from its chest and the Deku Shield from the shop;
- Mido steps aside;
- the Deku Tree's cutscene opens his mouth.

1. **The inventory and getting items** (done: see [GAME-03](GAME-03-road-to-deku-tree.md) milestone 1 and ADR 0019).
   - `SaveContext`'s inventory: items, equipment, upgrades, quest items, ammo. A new save as `Sram_InitNewSave` makes it.
   - `Item_Give` in full for what Kokiri Forest gives.
   - Player's get-item flow:
     - `Actor_OfferGetItem` / `Actor_OfferGetItemNearby`;
     - the get-item action with its animation;
     - `GetItem_Draw` and its draw functions (bakes, including `Gfx_TwoTexScroll`, which also fixes the placed recovery hearts);
     - the item's text.
   - `En_Box`, the chests.
   - Link's equipment on his model (no sword or shield until they're got). The B and C items on the HUD, with their icon bakes.
   - **Exit:** open the Kokiri Sword chest on a new save. Link holds it up, the text shows, and B gets the sword.
2. **Crawlspaces and the training area** (done: see [GAME-03](GAME-03-road-to-deku-tree.md) milestone 2 and ADR 0020).
   - Player's crawl.
   - `PIVOT_CRAWLSPACE` is ported, but `CRAWLSPACE`'s Subj4 camera isn't.
   - The training area's boulder and whatever else stands between the start and the sword.
   - **Exit:** a scripted run from Link's house to the sword chest.
3. **Mido and the shop** (done: see [GAME-03](GAME-03-road-to-deku-tree.md) milestone 3 and ADR 0021).
   - `En_Md`: blocking the path, his talk states, stepping aside with the sword and shield (`EVENTCHKINF` flags).
   - `En_Ossan`, the Kokiri shop only:
     - the browsing camera (`PIVOT_SHOP_BROWSING` is ported; the shopkeeper switches the viewpoint);
     - choosing, buying, and the rupee check;
     - the shelf items (`En_GirlA`).
   - **Exit:** buy the Deku Shield with the rupees from the bushes; Mido lets Link through. *(Done with the C's placed rupees instead of the bushes' random drops: 42 of them can be reached on foot.)*
4. **Cutscenes, first part** (done: see [GAME-03](GAME-03-road-to-deku-tree.md) milestone 4 and ADR 0022).
   - Import the cutscene scripts.
   - `csCtx`: the camera commands, Player's and the actors' cues (`npcActions`), text commands through the message box, transitions and terminators.
   - `Cutscene_HandleEntranceTriggers` and `cutsceneTrigger`.
   - Player's cutscene modes (`Player_SetCsActionWithHaltedActors`), which also completes `En_Wonder_Talk2`'s forced texts.
   - **Exit:** talking to the Deku Tree plays `gDekuTreeMeetingCs` to the end, yes plays `gDekuTreeMouthOpeningCs`, and his mouth opens (`EVENTCHKINF_05`); the scripted run from GAME-02 milestone 4 opens it the game's way, without the preset. *(Done with a new route, `NewSaveDekuTree`, from a new save through the shop and Mido; GAME-02's run keeps its preset.)*
5. **Navi** (done: see [GAME-03](GAME-03-road-to-deku-tree.md) milestone 5 and ADR 0023).
   - `En_Elf` as Link's fairy: following, the target reticle's `naviRefPos`, C-Up and her text.
   - The Kokiri children's fairies.
   - The game's opening: Navi waking Link (`ENTR_LINKS_HOUSE` with its cutscene layer). The cutscene system covers it now; the pack needs the scene layers 4 and up, and Player the cutscene modes the wake-up's script cues.
   - Navi's cues in the Deku Tree's talk (`npcActions[8]`).
   - **Exit:** a new save starts as the game does, and C-Up talks to Navi. *(Done: a new file plays the opening, four cutscene layers traced from their terminators; the Deku Tree's narration comes before the nightmare, so the wake-up is Link's house's layer 4, not the new file's 5.)*

**Phase exit:** a headless run from a new save to the Deku Tree scene, the game's way, replacing GAME-02 milestone 4's flag preset. *(Done in milestone 5: `Route::NewFileDekuTree`, 11748 frames from the file select's new file into the Deku Tree. GAME-02's run keeps its preset as a shortcut. Phase 4 is complete.)*

## Phase 5: audio (GAME-04, done)

**Goal:** music and sound effects, from the ROM through the pack.

**Why before Phase 6** (as planned): every dungeon system from here calls sounds, and each port had logged its calls as left out. This phase doesn't depend on Phase 4, so it could have gone earlier. `oot_extract::audio` already decodes the soundfonts, the VADPCM samples (checked bit for bit) and the sequences (a tick-accurate sequence player, to MIDI). The decomp's audio is about 18,000 lines (`audio_*.c`).

1. **The import and the synth** (done: see [GAME-04](GAME-04-audio.md) milestone 1, ADRs 0024 and 0025):
   - soundfonts, samples and sequences into the pack;
   - an `eng_audio` crate: an output device (for example `cpal`), and a synthesiser that follows `synthesis.c` (ADPCM playback, envelopes, pan, reverb);
   - an ADR on how faithful the mixer is.
   - *(Done whole: the pack holds the ROM's audio files as they are; the library, the sequence
     player included, and the microcode are ported, offline and through the device.)*
2. **The game's music** (done: see [GAME-04](GAME-04-audio.md) milestone 2, ADR 0026):
   - `seqplayer.c` driving the synth, including the game's IO ports;
   - the scene's music (`Environment_PlaySceneSequence`, `Audio_PlaySceneSequence` and the rest of `general.c`), the ambience (`Audio_PlayNatureAmbienceSequence`).
   - **Exit:** Kokiri Forest's music plays and loops like the game. *(Done headless, checked
     command for command against the C, and in the window with no `--music`: the scenes'
     sound settings from the pack, the spec changes, the fades between scenes, the forest's
     music resumed from a house, the night's ambience, the chests' and items' fanfares.)*
3. **Sound effects** (done: see [GAME-04](GAME-04-audio.md) milestone 3, ADR 0027):
   - `Audio_PlaySfxGeneral` and the sfx channels;
   - the calls the ported code already marks as left out: Player's footsteps, sword, text blips, doors, rupees, the HUD, the low-health alarm.
   - **Exit:** a scripted run's sfx log matches the calls in the C. *(Done: the Mido and shop
     run, two doors and the Deku Tree run's bushes, their requests and positions checked frame
     by frame against the C's arithmetic (`oot_actors --test sfx_route`), and the run's audio
     log as the golden `mido_shop_audio`. Phase 5 is complete.)*

## The cutscene phase (GAME-04b)

**Goal:** finish the cutscenes between Phase 5 and Phase 6 (decided 2026-10-01): what GAME-03's
cutscene system left out, the one-point cutscenes, and the camera modes still on the fallback.
On decomp `2f4c25d`'s names, migrated to main's in GAME-05 milestone 1. See
[GAME-04b-cutscenes.md](GAME-04b-cutscenes.md).

1. **One-point cutscenes** (done, ADR 0029): `z_onepointdemo.c`, the cameras' queue and
   `Camera_Finish`, `Camera_Unique9`, `Camera_Demo9`, `Camera_Demo5`; the crawlspace's exits
   (9601, 9602: BACKLOG #3), `En_Box`'s fall (4500) and attention calls; the system the Deku
   Tree's actors call.
2. **The camera modes on the fallback** (done): `Camera_Jump1`, `Camera_Jump2` (the ladder:
   BACKLOG #2), `Camera_Unique1`; the cutscenes' settings were already ported. `Camera_Battle1`
   waits for Phase 6's enemies.
3. **Cutscene audio** (done, BACKLOG #10): the scripts' music commands, `z_demo.c`'s own sounds,
   Player's cutscene modes (from `D_80854B18` and `D_80854E50` in the pack) with their voices and
   sounds.
4. **The rest of `z_demo.c`'s commands** (done): the misc actions on `envCtx` (rain,
   lightning...), the lighting override, the environment's lights every frame.
5. **Title cards** (done, BACKLOG #5): `TitleCard_InitPlaceName` and the scene-entry title
   cards, the place names' textures in the pack.
6. **The opening's nightmare** (done, BACKLOG #7): `En_Viewer` with the horses' skin
   skeletons (ADR 0030), `Bg_Spot00_Hanebasi`, the rain's and the bolts' draw.
7. **Cutscene polish** (done for the tests, BACKLOG #10): the splines, the letterbox, the
   narration's placement and Link's poses checked against the C by tests (no port bugs found);
   the look by hand is the user's.

## Phase 6: the Deku Tree (GAME-05, done)

**Decisions (2026-10-01, [ADR 0028](adr/0028-phase-6-master-quest-and-the-decomp-upgrade.md)):**
- **Master Quest.** The user chose the debug ROM's Deku Tree: gc-eu-mq-dbg stays the only ROM
  (ADR 0003), no second ROM version.
- **The decomp is upgraded first,** as the phase's first milestone, before any dungeon work:
  the upgrade brings named functions such as `Player_Action_*`, and the existing citations move
  to the new names through an address-based map.

1. **The decomp upgrade** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 1 and
   [ADR 0031](adr/0031-decomp-main.md), which supersedes ADR 0004's pin):
   - an address-based map from `2f4c25d`'s names to the new commit's: both build gc-eu-mq-dbg,
     so functions and statics pair by address in the two builds' symbol files (per overlay),
     fields by offset in the two commits' headers; generated by a tool, kept as a file, the
     unpaired ones listed for review;
   - the citations migrated by the map with a script: the code's identifiers named after the C,
     comments, tests, docs;
   - the importer migrated to the new decomp's layout (paths, the C and headers it parses, the
     asset XMLs and extraction), the pack's record names that change renamed (a format bump);
   - **Exit:** every test passes and the goldens are the same bytes, on the new commit.
     *(Done on zeldaret/oot main at `52a510f`: every test and render the same; the 30 traces
     that carry C names re-recorded, each the old one's bytes once renamed through the map.)*
2. **Damage and health** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 2 and
   [ADR 0032](adr/0032-damage-death-and-the-game-over-stand-in.md)):
   - Player taking damage: knockback, invincibility frames, `Health_ChangeBy`. *(Mostly done in GAME-03 milestone 2 for the boulder, ADR 0020: the body hit, the stagger, the knockdown, the invincibility timer, the fall damage. Left: kinds 3 and 4 (frozen, shocked), the hit while swimming, burning, the red flash.)* *(Done: all of them.)*
   - death and game over (`Play_TriggerVoidOut` exists); *(done: `func_80836448`, the death animations, `z_game_over.c`, a bottled fairy's revival, and the game over menu's states as a stand-in, undrawn, "Continue? Yes" as the C)*
   - enemy damage tables (`CollisionCheck_ApplyDamage`, `DamageTable`). *(Done, with `Actor_ApplyDamage`, the colour filter and the drop flag.)*
   - **Exit:** the dummy and a Deku Baba hit Link. *(Done: the dummy hurts Link with each hit kind; a Deku Baba bites him and he cuts its stem, the golden `deku_baba`.)*
3. **The first enemies,** split in two (decided 2026-10-05):
   - **3a, combat basics** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 3a,
     [ADR 0033](adr/0033-effects.md) and [ADR 0034](adr/0034-guard-battle-camera-and-the-first-enemies.md)):
     - Player's guard with the shield (R: the shield's collider, blocking and deflecting; BACKLOG #4), which the Deku Scrubs' nuts need;
     - enemy targeting and `Camera_Battle1`;
     - the effects (`EffectSs` and `z_effect.c`'s: dust, fragments, hit marks, the flames, the death flame), with `En_Dekubaba`'s *(ported in milestone 2, whole but its effects)*;
     - the MQ Deku Tree's layer-0 enemies that need nothing more: `En_Firefly` (Keese, 7 placed) and `En_Karebaba` (withered Deku Baba, 5);
     - drops on death;
     - **Exit:** Link blocks a hit with the shield, and fights a Keese and a withered Deku Baba in the Deku Tree with the battle camera; the kills show their effects and drop items. *(Done: `Route::Combat`, the golden `combat`; `En_Nutsball` pulled forward from 3b for the deflection, `Item_Shield` ported.)*
   - **3b, the rest of the MQ Deku Tree's enemies** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 3b and [ADR 0037](adr/0037-the-deku-trees-other-enemies.md)): `En_St` (2 placed), `En_Sw` (Skullwalltula and Gold Skulltula, 7), `En_Hintnuts`, `En_Dekunuts` and `En_Shopnuts` (3, 2, 1), `En_Goma` (eggs and larvae, 28, pulled forward from milestone 6).
     - **Exit:** from debug starts in their rooms, Link bounces a Deku Scrub's nut back to knock it out and catches it, kills a Skullwalltula, a Gold Skulltula (its token collected) and a Skulltula, and a Gohma egg hatches and its larva is killed. *(Done: `Route::Scrub`, the golden `scrub`, for the Mad Scrub; the rest by C-derived tests from debug starts in their rooms and by hand. `En_Dns`, the salesman a caught Business Scrub becomes, and `En_Si`, the token, ported whole; `EffectBlure`, the Skulltula's trail, still not.)*
4. **Dungeon mechanics,** split in three (decided 2026-10-06; every actor ported whole, the
   triggers Link can't use yet injected in the tests):
   - **4a, doors, switches, torches and webs** (done: see [GAME-05](GAME-05-deku-tree.md)
     milestone 4a and ADRs [0038](adr/0038-sliding-doors-and-room-travel.md),
     [0039](adr/0039-quakes.md), [0040](adr/0040-switches-torches-webs-and-the-map-data.md)):
     `Door_Shutter` with Player's sliding door and small keys; `z_quake.c`; `Obj_Switch`,
     `Obj_Syokudai`, `Bg_Ydan_Sp`, `Elf_Msg`, `Elf_Msg2`; the map and compass as data (drawn in
     milestone 5); a debug start per room.
     - **Exit:** from room 0's top floor, the switch burns the web and Link goes through room 10's
       sliding door, which bars behind him. *(Done: `Route::Shutter`, the golden `shutter`.)*
   - **4b, the Deku Stick and the props** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 4b
     and ADRs [0041](adr/0041-the-deku-stick-and-the-item-buttons.md),
     [0042](adr/0042-the-deku-trees-props-and-the-fragments.md)): the Deku Stick pulled forward
     from milestone 5 (used from C-Left; Player's item buttons whole), `Bg_Ydan_Hasi` and
     `Bg_Ydan_Maruta` whole, `Obj_Kibako2` with `Effect_Ss_Kakera` (and `En_Kusa`'s, `En_Ishi`'s
     and `En_Goroiwa`'s pieces), `Obj_Lift`.
     - **Exit:** a Deku Stick lit at a golden torch burns the web over room 1's door. *(Done:
       `Route::Stick`, the golden `stick`; the stick is swung with its C button, B taking the
       sword out, as in the C.)*
   - **4c, pushing and Master Quest's extras** (done: see [GAME-05](GAME-05-deku-tree.md)
     milestone 4c and ADRs [0043](adr/0043-push-and-pull.md),
     [0044](adr/0044-master-quests-extras-and-room-travel.md)): Player's push and pull (the heavy
     block's lift logged past its checks), `Obj_Oshihiki` and `Obj_Makeoshihiki`, `Bg_Haka`,
     `Obj_Timeblock` (the song injected), `Obj_Bombiwa`; the connections milestone 4 opens walked
     through from one start (`--test travel`).
     - **Exit:** room 3's block pushed off into the pit and climbed. *(Done: `Route::Push`, the
       golden `push`. Room 7's gravestones are sunk too low in Master Quest to hold on to.)*
5. **Items in use,** split in three (decided 2026-10-07; see [GAME-05](GAME-05-deku-tree.md)
   milestone 5's survey):
   - **5a, the Fairy Slingshot and Deku nuts** (done: see [GAME-05](GAME-05-deku-tree.md)
     milestone 5a, ADRs [0045](adr/0045-the-fairy-slingshot-first-person-and-deku-nuts.md) and
     [0046](adr/0046-en-arrow-the-nuts-stun-and-the-flash.md)): Player's bow and slingshot
     actions, first person (C-Up's look and the aim) with `Camera_Subj3`, the nut's throw,
     `heldActor`; `En_Arrow` whole (the adult arrows' trail and the magic arrows logged),
     `En_M_Fire1`, `Effect_Ss_Stone1`, the screen's flash; Start's stand-in putting nuts and the
     slingshot on C buttons (replaced by the menu in 5b-1); the eye switches, room 2's ladder and room 10's chest for real; the
     room travel test round rooms 1 to 8; BACKLOG #18.
     - **Exit:** from room 1, the slingshot aimed in first person at the eye, the door to room 2
       unbarred, and through it. *(Done: `Route::Slingshot`, the golden `slingshot`.)*
   - **5b, the pause menu:** its frame (`KaleidoSetup`, the open and close, the pages' box and
     turns, the cursor, the name and info panels), the item page and the dungeon map page (with
     its marks) whole, the equipment and quest pages' backgrounds with their contents logged; the
     C buttons equipped from it, replacing the item half of Start's stand-in; the game over
     screens drawn. Split in two (decided 2026-10-07):
     - **5b-1, the frame and the item page** (done: see [GAME-05](GAME-05-deku-tree.md) milestone
       5b-1, [ADR 0047](adr/0047-the-pause-menu.md)): the frame whole, the item page whole with
       the equip's flight, the other pages' backgrounds (the world map's too, outside dungeons),
       the menu at `R_UPDATE_RATE` 2, the scene behind kept, the HUD's pause part; Start's
       stand-in down to its equipment half.
       - **Exit:** in the Deku Tree, the menu opened, the slingshot equipped on C-Right from the
         item page, the page turned and the menu closed. *(Done: `Route::Pause`, the golden
         `pause` with `pause_item` and `pause_map`.)*
     - **5b-2, the dungeon map page and the game over** (done: see [GAME-05](GAME-05-deku-tree.md)
       milestone 5b-2, [ADR 0048](adr/0048-the-pause-map-and-the-game-over.md)):
       `z_kaleido_map.c`'s dungeon map whole with its loads and `z_lmap_mark.c`'s marks
       (`gPauseMapMarkDataTable` and `map_48x85_static` in the pack), the room maps' run-time
       palette through a per-draw image in the engine; the game over screens drawn through
       `KaleidoScope_Draw`.
       - **Exit:** the map page from a debug start inside the Deku Tree (the visited floors, the
         chests' marks with the compass, a floor changed with the stick); the game over's
         screens in a golden. *(Done: `Route::DungeonMap`, the golden `dungeon_map` with
         `dungeon_map_1f` and `dungeon_map_2f`; `Route::GameOver`, the golden `game_over` with
         `game_over_message`, `game_over_save` and `game_over_continue`.)*
   - **5c, saving** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 5c,
     [ADR 0049](adr/0049-saving.md)): `z_sram.c`'s save and load whole, the slots on disk in the
     C's layout, the pause menu's save prompt, the game over's `Sram_WriteSave`, and a stand-in for
     the file select's load (`--file N`, F5's reset, `ootx sram`).
     - **Exit:** from a debug start inside the Deku Tree, the slingshot equipped from the menu, B,
       Yes: the save written, the menu closed; the console's reset loads the file back with its
       state restored. *(Done: `Route::Save`, the golden `save` with its SRAM image,
       `save_prompt_turn` and `save_prompt`.)*
6. **Gohma,** split in three (decided 2026-10-09; see [GAME-05](GAME-05-deku-tree.md) milestone
   6's survey):
   - `Boss_Goma` (about 2,100 lines); her eggs and larvae (`En_Goma`) are pulled forward to 3b;
   - the boss room's camera and cutscenes (Phase 4's cutscene system);
   - the heart container, the blue warp out.
   - **Exit:** a scripted run through the Deku Tree to Gohma's defeat.
   - **6a, Queen Gohma** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 6a,
     [ADR 0050](adr/0050-queen-gohma.md), [ADR 0051](adr/0051-textures-replaced-by-source-and-the-object-ram.md)):
     `Boss_Goma` whole (her intro and death are her own actions, on her own sub camera), her draw
     and decay, `TitleCard_InitBossName`, `Item_B_Heart` whole, the slab's quake on her camera,
     Player's jump slash pulled forward.
     - **Exit:** from the boss room's entrance, her intro, the fight, her death, the heart
       container taken. *(Done: `Route::Gohma`, the golden `gohma` with `gohma_title` and
       `gohma_decay`.)*
   - **6b, the blue warp** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 6b,
     [ADR 0052](adr/0052-the-blue-warp.md)): `Door_Warp1` for this ROM's Deku Tree (the child warp;
     the destination warp at Kokiri Forest's arrival), one-point 9703, Player's blue-warp arrival
     (start mode 2), Kokiri Forest's first cutscene to its terminator.
     - **Exit:** into the warp, out with the Kokiri Emerald, the arrival and the Deku Tree's texts.
       *(Done: `Route::BlueWarp`, the golden `blue_warp` with `blue_warp_float` and
       `blue_warp_forest`.)*
   - **6c, through the Deku Tree** (done: see [GAME-05](GAME-05-deku-tree.md) milestone 6c):
     room 11's exit floor; the full run as a test (the travel test carried on through her fight
     and the warp); a short golden from room 9 into the boss room; `--clear` for the game.
     - **Exit:** from room 9's debug start through room 11 into her room. *(Done:
       `Route::BossRoom`, the golden `boss_room` with `boss_room_door`; the whole Deku Tree in
       `travel.rs`.)*

## Phase 7: the road out of the forest (GAME-06)

**Goal:** from the Deku Tree's death into Hyrule Field: the Kokiri Emerald's cutscene chain
played to its end (BACKLOG #23), Saria's goodbye on the bridge, and Hyrule Field with its clock,
its owl and its enemies. See [GAME-06](GAME-06-out-of-the-forest.md)'s survey. Decided
(2026-10-09): the split below; `Demo_Effect`, `Demo_Kankyo` and `En_Owl` whole, the parts of
`Demo_Sa` and `Obj_Bean` this phase reaches; the lens flare's occlusion through an engine depth
probe. And:
- **Kakariko and Castle Town move to Phase 8:** about 52,000 lines of actors with Hyrule Castle.
  Phase 7 ends with Hyrule Field's exits into them working, their actors placeholders.
- **The day and night debt** (cross-cutting, "World") is milestone 3, before the field's enemies.

1. **The cutscene chain after part 1,** in two:
   - **1a, the emerald and Farore's light** (done: see [GAME-06](GAME-06-out-of-the-forest.md)
     milestone 1a, [ADR 0053](adr/0053-demo-effect-curve-skeletons-and-vertices-by-source.md)):
     `Demo_Effect` (2,092 lines) whole with the curve skeletons its time warps draw, Kokiri
     Forest's cutscene layers 4 and 6; a debug start by cutscene index.
     - **Exit:** the emerald over Link through the Deku Tree's death to `ENTR_KOKIRI_FOREST_11`.
       *(Done: `Route::Emerald`, the golden `emerald` with `emerald_light`, `emerald_floats` and
       `farore`.)*
   - **1b, the creation** (done: see [GAME-06](GAME-06-out-of-the-forest.md) milestone 1b,
     [ADR 0054](adr/0054-the-128-skies-and-demo-kankyo.md)): `Demo_Kankyo` (1,015 lines) whole,
     `Bg_Spot09_Obj`, `Bg_Spot16_Doughnut`, Gerudo Valley's and Death Mountain Trail's draw
     configs, the outdoor skies (the cutscene map's, the overcast sunset's, and the normal sky at
     the scene's time).
     - **Exit:** the whole chain, part 1 to `ENTR_KOKIRI_FOREST_11`, as a test; short goldens.
       *(Done: `Route::BlueWarp` carried on by `Route::Creation` in a test; the golden `creation`
       with `creation_rain`, `creation_rocks` and `creation_nayru`.)*
2. **Leaving the forest** (done: see [GAME-06](GAME-06-out-of-the-forest.md) milestone 2,
   [ADR 0055](adr/0055-leaving-the-forest.md)): Kokiri Forest after the emerald, the Lost Woods
   bridge (`Demo_Sa`'s bridge, `Obj_Bean`'s soft soil), the Fairy Ocarina, Hyrule Field's intro,
   `En_Owl` (1,437) whole.
   - **Exit:** out of the forest, Saria's goodbye, the field's intro, the owl's talk.
     *(Done: `Route::Farewell`, the golden `farewell` with `farewell_bridge`, `farewell_ocarina`
     and `farewell_owl`.)*
3. **The clock, day and night** (done: see [GAME-06](GAME-06-out-of-the-forest.md) milestone 3,
   [ADR 0056](adr/0056-the-clock.md)): time passing (`Environment_Update`), the sky by the time of
   day (`SKYBOX_NORMAL_SKY`), the sun and the moon, the lens flare; the lights and the music by
   time.
   - **Exit:** Hyrule Field from dusk into night.
     *(Done: `Route::Dusk`, the golden `dusk` with `dusk_sunset` and `dusk_bridge`.)*
4. **Hyrule Field** (done: see [GAME-06](GAME-06-out-of-the-forest.md) milestone 4,
   [ADR 0057](adr/0057-hyrule-fields-actors.md)): its props (`En_Wood02`, `Obj_Mure2`,
   `En_A_Obj`, `Door_Ana`) and enemies (`En_Peehat`; `En_Encount1` and `En_Skb`, the
   Stalchildren; `En_Part` and `BodyBreak`), each whole; the enemy music.
   - **Exit:** a Peahat by day, Stalchildren by night, the way to Castle Town and Kakariko.
     *(Done, in two runs: `Route::Field`, the golden `field` with `field_peahat`;
     `Route::FieldNight`, the golden `field_night` with `field_night_fight`.)*

BACKLOG #24 stays open: none of milestone 1's lists read `LOD_FRACTION`; #21 and #22 (the title
screen and the file select, about 6,400 lines) can follow milestone 3, as the menus' own
milestone; #1 goes with Phase 8's prerendered houses and shops.

## Phase 8: Castle Town, Hyrule Castle and Kakariko (GAME-07)

**Goal:** child Link's first pass beyond Hyrule Field: Castle Town by day and night, Hyrule Castle
(Malon, Talon and the crates, the guards, Zelda, her letter, Impa's escort and Zelda's Lullaby),
Kakariko and its graveyard by day and night, and the ocarina. See
[GAME-07](GAME-07-castle-town-and-kakariko.md)'s survey. Decided (2026-10-11): the split below;
every actor whole except `En_Ta` (the castle's and Kakariko's parts), `Demo_Im` (types 3 to 5),
the actors killed for a child (`En_Xc`, `Demo_6K`, `Bg_Toki_Swd`, `En_Go2`: their inits' checks)
and the ocarina's scarecrow, memory game and frogs; the shops and games left out; the culling
volume (BACKLOG #32) first, `En_Bom` (#33) with the bombs.

1. **Castle Town:** the culling volume; the entrance, the market, the back alley, the Temple of
   Time's outside by day and night, the Temple's altar; `En_Hy`, `En_Heishi4`, `En_Dog`, `En_Ma1`,
   `En_Mu`, `En_Niw_Girl`, `En_Tg`, `En_Gs`, `Obj_Mure`; `Camera_Fixed1`; BACKLOG #1.
   - **Exit:** from the entrance by day through the market to the castle's road and its intro.
   - **Done** (2026-10-11, ADR 0058, ADR 0059): with `En_Niw`, `En_Butte`, `En_Wonder_Talk`,
     `Bg_Toki_Hikari`, `Bg_Mjin`; the Temple's room 0 (behind the Door of Time, `En_Xc`,
     `Demo_6K`, `Bg_Toki_Swd`) left for the door (BACKLOG #37). `Route::Market`, the `market`
     golden.
2. **Hyrule Castle's grounds:** the Weird Egg and its hatch at dawn, Player's exchange items,
   `En_Ta`, `Bg_Spot15_Rrbox`, `Bg_Spot15_Saku`, `En_Heishi2`, `En_Heishi3`, `Obj_Tsubo`,
   `En_Light`, `TransitionCircle`, `Camera_Jump3`.
   - **Exit:** Talon woken with the Cucco, the crates pushed into the moat, through the drain.
   - **Done** (2026-10-12, ADR 0060): with `Bg_Gate_Shutter` (pulled forward from 5) and
     `Camera_UpdateWater`'s settings half; the hatch was already ported. `En_Ta`'s ranch by day
     (BACKLOG #44), the eye under water (#43), the bottles', spells' and ocarina's uses (#45)
     logged. `Route::Castle` and `Route::CastleCaught`, the `castle` and `castle_caught` goldens.
3. **The ocarina:** Player's ocarina, the message system's ocarina modes, `AudioOcarina`, the
   songs' effects (`Oceff_*`), `Shot_Sun` (BACKLOG #36), the Sun's Song's time, the cutscenes'
   ocarina text.
   - **Exit:** in Hyrule Field with the songs: Saria's Song, the Sun's Song, a fairy at `Shot_Sun`.
   - **Done** (2026-10-12, ADR 0061): with `En_Okarina_Tag`, `Oceff_Wipe4` and the actor update's
     ocarina freeze (`ACTOR_FLAG_UPDATE_DURING_OCARINA`); `En_Ma1`'s Epona's Song lesson, `En_Md`'s
     check, `En_Gs`, `En_Box`'s song chests, `Obj_Timeblock` and `En_Kanban` hear the songs. The
     scarecrows, the memory game and the frogs (BACKLOG #47), the fire arrows' item (#48), magic
     (#49), the quest page's songs (#50), `En_Weather_Tag` and `En_Horse` (#51) left out.
     `Route::Ocarina`, the `ocarina` golden.
4. **The guards' courtyard and Zelda:** `Camera_Special6`, `En_Heishi1`, `En_Zl4`, the legend,
   the letter, `Demo_Im`, the lullaby taught, Hyrule Field's layer 12; the draw configs of the
   castle (its water, BACKLOG #46), the courtyards and the Temple of Time.
   - **Exit:** past the guards, Zelda's talk to the letter, Impa's lullaby and goodbye.
   - **Done** (2026-10-13, ADR 0063): with `En_Ex_Ruppy` (the courtyard's rupees) and
     `SDC_CALM_WATER`; Player's talk with text -1 and root motion for any actor. `Demo_Im`'s other
     types (#53), the diving game's rupees (#54) and the Temple of Time's lens flare (#55) logged.
     `Route::Zelda` (from the drain, past the patrols, Zelda, the legend, the letter, the lullaby,
     the field to `ENTR_HYRULE_FIELD_17`) and `Route::ZeldaTalk`, the `zelda` golden.
5. **Kakariko:** Player's lift, carry and throw, `En_Niw`, `En_Niw_Lady`, the gate, the people,
   the interiors; the draw configs of Kakariko, the grottos and the guard house (#46).
   - **Exit:** cuccos into Anju's pen, the letter to the guard, the gate opened.
   - **Done** (2026-10-13, ADR 0064): one milestone in steps (the user's call). Player's lift,
     carry, throw and put-down with the cucco's glide (BACKLOG #4), `En_Ishi`'s and `En_Kusa`'s
     lifted states; `En_Niw_Lady`, `En_Attack_Niw`, `En_Daiku_Kakariko`, `En_Toryo`, `En_Hs2`,
     `En_Ani`, `Bg_Spot01_Fusya`, `_Idohashira`, `_Idomizu`, `_Objects2`; the interiors' `En_Cow`,
     `En_Fu`, `Bg_Relay_Objects`, `En_Ssh`, `En_Sth` (the House of Skulltula and Kakariko's potion
     shop moved into the phase); `Actor_CleanupContext` on a scene change; an overlay's animations
     in the importer. Kakariko's intro was already playing. The grotto's actors (#58), the heavy
     block (#56) and the adult's Master Sword (#57) left out. `Route::Kakariko`, the `kakariko`
     golden.
6. **The graveyard:** Dampé's dig, the royal tomb, the ReDeads, the Sun's Song; the draw configs
   of the graves, the royal tomb and the fairy fountain (#46).
   - **Exit:** a dig at 18:00, the lullaby at the tomb, the Sun's Song learned.
   - **Done** (2026-10-13, ADR 0065): one milestone in steps (the user's call). Player's grab and
     auto lock-on; `En_Cs`, `En_Tk`, `En_It`, `Bg_Spot02_Objects`, `En_Rd`, `Bg_Breakwall`,
     `En_Weather_Tag` (BACKLOG #51), `Obj_Makekinsuta`, `Object_Kankyo`'s types 2 and 4,
     `EffectSsBomb2` (#40); the switch flags' wrap past 0x3F. The intro and the Sun's Song's chain
     were already playing; Dampé is out only after 18:00, so the run starts at 18:30.
     `En_Po_Relay` left for Phase 14 (a child can't open Dampé's grave). The masks (#39), the bugs
     (#59) and the lens (#60) left out. `Route::Graveyard`, the `graveyard` golden.

## Phase 9: the Lost Woods, the Sacred Forest Meadow and Lon Lon Ranch (GAME-08)

**Goal:** child Link with Zelda's letter through the Lost Woods to the meadow, its Wolfos and
maze, and Saria's Song; the woods' Skull Kids and scrubs; Lon Lon Ranch's Epona's Song, the young
Epona, Ingo and Talon's house. Exit: Saria's Song learned. See
[GAME-08](GAME-08-woods-meadow-and-ranch.md)'s survey. Decided (2026-10-14): the split below, the
exit first; every game and shop of the phase's scenes in (the Skull Kids' memory game, the target
scrub, Talon's milk and Super Cucco game with the HUD timer); GAME-07's scope rule (every actor
whole, a child-killed one its init's checks); the coverage table's corrections. Before it, the
actor schema (ADR 0066, BACKLOG #52) for all ported actors.

1. **The meadow and Saria's Song:** `En_Sa` (with Kokiri Forest's greeting), `En_Wf`,
   `Bg_Spot05_Soko`; the woods' maze walked.
   - **Exit:** from the Lost Woods' entrance with the letter to Saria's Song learned.
   - **Done** (2026-10-14, ADR 0067): with the `z_actor.c` helpers the Wolfos calls,
     `CollisionCheck_CylSideVsLineSeg`, `EffectSsKiraKira`; Saria's greeting now plays on every
     new save (the routes, tests and presets brought up to date). `Route::Meadow`, the `meadow`
     goldens.
2. **The Lost Woods' Skull Kids and scrubs:** `En_Skj` with `En_Skjneedle`, the ocarina's memory
   game (BACKLOG #47's Skull Kid part), `En_Dnt_Nomal` with `En_Ex_Item`, `EffectSsExtra` and
   one-point cutscene 4140.
   - **Done** (2026-10-14, ADR 0069): the four actors whole (the Skull Mask trade waiting on masks),
     the memory game in the ocarina and the message box, `meadow-saria` (milestone 1's end, with
     the slingshot). `Route::LostWoods` (Saria's Song on the stump, three bullseyes, the memory
     game's three rounds), the `lost_woods` goldens; `En_Okuta` moved to Phase 11 (it never spawns
     in the woods).
3. **Lon Lon Ranch:** Epona's Song's cutscene end (destination 94), the two draw configs,
   `En_Horse_Link_Child`, `En_Horse_Normal`, `En_In` (the child's part), `En_Crow`, `En_Ma2`'s
   init, `En_Ta`'s ranch house with the milk and the Super Cucco game, the HUD timer (#44).
   - Split in two steps (the user's call, 2026-10-14). **Step (a) done** (2026-10-14, ADR 0070):
     the foal and the horses (the first live skin skeletons), the child's Ingo (the adult's race a
     placeholder), `En_Ma2`'s init, the Guays, the two draw configs, `Interface_SetTimer` and the
     timers' state; `lost-woods-games` (milestone 2's end). `Route::Ranch` (Malon's lesson, the
     song's scene and back by destination 94, the foal called), the `ranch` goldens. **Step (b)
     done** (2026-10-15, ADR 0071): the HUD's timers whole (the main and sub timers, the hot rooms'
     and the water's, their draw), `En_Ta`'s ranch house (the milk, the Super Cucco game; #44
     closed); `Route::Ranch` on into Talon's house, the game won and the bottle of milk.

**Phase 9 done** (2026-10-15): its three milestones built, and its exit review (GAME-08's last
section): the `ranch-epona` preset (Phase 10's start), what's left and where. Phase 10 starts
with its survey.

## Phase 10: Death Mountain, Goron City and Dodongo's Cavern (GAME-09)

**Goal:** child Link up Death Mountain Trail to Goron City, Darunia's dance and the Goron's
Bracelet, the bomb flower that opens Dodongo's Cavern (Master Quest), the cavern, King Dodongo and
the Goron's Ruby; the summit's Great Fairy and the magic meter. Exit: the Goron's Ruby. See
[GAME-09](GAME-09-death-mountain-and-dodongos-cavern.md)'s survey. Decided (2026-10-15, the
recommendations taken): the split below, the critical path first and the exit at milestone 4;
`En_Go2`'s adult types and `Demo_Du`'s other cutscenes placeholders chosen at init, `En_Gm` whole;
`En_M_Thunder` with the magic meter; `EffectBlure` with `En_Zf`; the coverage table's corrections.

1. **Bombs:** `En_Bom`, `En_Bombf`, `Effect_Ss_G_Spk`, Player's explosives, BACKLOG #33.
   - **Done** (2026-10-15, ADR 0072): with Player's explosive in hand through a predicted handle,
     `func_8002F5C4`, `holds_actor` and a square-on item offer fixed; the `cavern-bombs` preset (a
     debug start). `Route::Bombs` (the west flower put down at the cavern's entrance wall, its
     opening; a bomb from the bag by the east flower), the `bombs` goldens.
2. **The trail and Goron City to the bracelet, and the cavern's mouth:** `En_Go2`, `En_Du`,
   `Bg_Spot18_*`, `En_Gm`, `Obj_Hamishi`, `Obj_Hsblock`, `En_Tite`, `Bg_Bombwall`,
   `Bg_Spot16_Bombstone`, `En_Hata` and the scene's wind, `SDC_GORON_CITY`, one-points 4180, 4210,
   4220, the child's bracelet drawn.
   - **Done** (2026-10-15, ADR 0073), in one step: the thirteen overlays whole (the adult's Gorons
     placeholders chosen in `En_Go2`'s init), the room's wind (pack format 29), one-points 4180,
     4210, 4220 and 4221, `SDC_GORON_CITY`, the bracelet on Link, Player's fidget by the room's
     environment. Two short runs from debug starts (`Route::Darunia`: the lullaby at his door,
     Saria's Song, the bracelet; `Route::Boulder`: the cliff's Bomb Flower thrown onto the
     boulder) with the `darunia` and `boulder` goldens; the presets `goron-bracelet` and
     `cavern-open` (milestone 3's start). The story chain up the trail is hand-tested.
3. **Dodongo's Cavern's route:** `SDC_DODONGOS_CAVERN`, `Bg_Dodoago`, `Bg_Ddan_Jd`, `En_Dodojr` and
   what the way the collision search finds needs; one-points 3060, 3065, 3380.
   - **Done** (2026-10-16, ADR 0074), route B (the user's call): the collision search's way (the
     shortcut pillar is the only way up; room 3's levels apart; the bag from room 4), and with it
     `En_Zf` (with `EffectBlure`, wired to Link's sword, the Skulltulas and the arrows), `En_Poh`,
     `En_Am`, `Obj_Elevator`, `Bg_Hidan_Curtain`, `Bg_Gnd_Darkmeiro` from milestone 5. Four short
     runs from debug starts (the pillar, the bag, the skull's eyes and jaw, the gravestone and
     the fall; the fights are hand-tested) with their goldens; the presets `cavern-upper`,
     `cavern-bomb-bag`, `cavern-jaw` and `cavern-boss` (milestone 4's start) checked against them.
     No way found for a child to bring fire to room 10's web (GAME-09's milestone 3 section):
     dropped at the hand test, `cavern-web` starts past it. The loop is hand-tested.
4. **King Dodongo and the Goron's Ruby:** `Boss_Dodongo`, `En_Bdfire`, the magma effects, the room
   texture's write, `Play_SetCameraAtEyeUp`, `MREG(64)`; `Door_Warp1`'s branch, `Demo_Du`, `Demo_Go`.
   The phase's exit.
   - **Done** (2026-10-17, ADR 0075): **the phase's exit is met** (the Goron's Ruby). The room RAM
     the game writes as play state (the lava floor's texture), the camera's up modelled,
     `MREG(64)`'s fill; `Boss_Dodongo`, `En_Bdfire`, `Effect_Ss_G_Magma`, `G_Magma2`, `Demo_Du`'s
     Ruby part (its other cutscenes placeholders) and `Demo_Go` whole; `Door_Warp1`'s branch. Two
     short runs (`Route::KingDodongo`: the floor bombed, the drop, his intro; `Route::Ruby`: after
     his defeat the heart, the warp and `gDMTGoronRubyCs` to the trail's spawn 5), the presets
     `cavern-boss-fight` and `goron-ruby` (milestones 5 and 6's start). The fight is hand-tested.
5. **The rest of the cavern:** `En_Dodongo`, `En_Vm`, `En_G_Switch`, `Bg_Ddan_Kd`; `D_Fire`,
   `Dead_Dd`, `Dead_Ds`; one-point 3050; rooms 2, 5, 6, 13, 14 and 15 (`En_Zf`, `En_Am`, `En_Poh`,
   `Bg_Hidan_Curtain`, `Bg_Gnd_Darkmeiro` and `Obj_Elevator` came in milestone 3).
   - **Done** (2026-10-19, hand-tested 2026-10-20, ADR 0080), last: `Bg_Ddan_Kd` and one-point 3050, `En_G_Switch` (every
     type; the shooting gallery's writes logged, #98), `En_Dodongo`, `En_Vm`, `Effect_Ss_D_Fire`,
     `Dead_Dd`, `Dead_Ds`; rooms 2, 5, 6, 13, 14 and 15 whole (every placement ported: Phase 10's
     33 of 33). Four short runs from debug starts (`Route::FallingStairs`: room 2's Bomb Flowers
     drop the stairs; `Route::SilverRupees`: the last rupee and door 17; `Route::ShieldChest`: room
     14's gravestone and chest; `Route::TimeBlocks`: room 15's Song of Time), the presets
     `cavern-stairs`, `cavern-rooms` (the milestone's end) and `cavern-song-of-time`. The fights
     are hand-tested.
6. **The summit:** the magic meter, `Bg_Dy_Yoseizo`, `En_Dy_Extra`, `En_M_Thunder`,
   `SDC_GREAT_FAIRYS_FOUNTAIN`, the return entrances, `En_Encount2`, `En_Fire_Rock`; the owl's
   flight to Kakariko.
   - **Done** (2026-10-17, ADR 0076), before milestone 5 (the user's order): the magic meter
     (`oot_game::magic`: the state on the save context, `Interface_Update` runs it, `Magic_DrawMeter`
     after the rupees; #49 closed), `Bg_Dy_Yoseizo` whole with every reward, `En_Dy_Extra`, the
     orange warp, one-points 8603 and 8604; the return entrances (all of `sReturnEntranceGroupData`);
     Player's charged spin and the C's combo window, `En_M_Thunder`, `Eff_Dust`, `Camera_Battle4`;
     `En_Encount2`, `En_Fire_Rock`; `SDC_GREAT_FAIRYS_FOUNTAIN`. Three short runs from debug starts
     (`Route::FairyWall`: a bomb breaks the Great Fairy's wall; `Route::GreatFairy`: the lullaby,
     her cutscene, the magic, out by the return entrance; `Route::SummitOwl`: the owl's flight to
     Kakariko) and the presets `fairy-fountain` and `fairy-magic` (milestone 5's and Phase 11's
     start). The climb through the falling rocks is hand-tested.

**Phase 10 done** (2026-10-20): its six milestones built and hand-tested, and its exit review
(GAME-09's last section): `fairy-magic` is Phase 11's start (checked against the owl run's end, the
story's last), what's left and where. Phase 11 starts with its survey.

## Phase 11: Zora's River, Zora's Domain, Lake Hylia, Zora's Fountain and Jabu-Jabu (GAME-10)

**Goal:** child Link up Zora's River (the waterfall's lullaby) to Zora's Domain, the silver scale
from the diving game, Ruto's letter from Lake Hylia's bottom, King Zora moved aside, a fish for Lord
Jabu-Jabu, Jabu-Jabu's Belly (Master Quest) and its boomerang, Barinade, Ruto's warp and the Zora's
Sapphire; then Zelda's escape, the Ocarina of Time in the moat and the Song of Time. Exit: the
Zora's Sapphire, then the Ocarina of Time. See [GAME-10](GAME-10-zoras-waters-and-jabu-jabu.md)'s
survey. **Surveyed and proposed** (2026-10-20): nine milestones (water and bottles; the river and
the Domain to the silver scale; the letter, King Zora and Jabu-Jabu's mouth; Jabu-Jabu to the
boomerang; to Barinade; Barinade and the Sapphire; the moat; the rest of Jabu-Jabu; the frogs and
scarecrows). Decided 2026-10-20 (every recommendation). **Milestone 1 done** (2026-10-20: Player's
water and bottles, the fish and bugs, the water's effects, the currents, the underwater view);
milestones 2's and 3's actors merged (the river, the Domain, the diving game, the lake, the lab,
the fountain, Jabu-Jabu) with `game-zora.bat`'s debug starts; **milestones 2 and 3 done**
(2026-10-21: hand-tested, the river, King Zora and Jabu-Jabu runs). **Milestone 4**
(2026-10-21, ADR 0082: Jabu-Jabu's draw config with the walls' pulse as site matrices, pack format
30; its switches, lifts and spouts; the six enemies; the runs to room 1's yellow switch and the
boomerang), **done** (2026-10-21, hand-tested; the elevator's foot shadows fixed). **Milestone 5**
(2026-10-21, ADR 0083: `En_Ru1` whole with Player's Ruto checks, `En_Ba`, `En_Bx`, `Obj_Kibako`,
`Obj_Roomtimer`; the silver scale in the story's presets, BACKLOG #102; the runs to Ruto's meeting
and the cow, room 11's stick to room 7's web, and room 5's crystal to Barinade's room) **done**
(committed 2026-10-21; hand-tested 2026-10-22). **Milestone 6**
(2026-10-21, ADR 0084: `Boss_Va` whole, `Door_Warp1`'s Ruto warp, the fountain's Sapphire cutscene;
the runs Barinade's intro and the Sapphire; the three stones at `zora-sapphire`) **done** (committed
2026-10-21; hand-tested 2026-10-22: the blue warp's look fixed, the renderer's static
`LOD_FRACTION`, pack format 31). **Milestone 7** (2026-10-21, ADR 0085:
`Item_Ocarina` and `En_Ganon_Mant` whole; the runs Zelda's escape and the Ocarina of Time through
the Temple of Time's Song of Time; the presets `zelda-fled` and `ocarina-of-time`, Phase 12's
start) **done** (committed 2026-10-21; hand-tested 2026-10-22). **Milestone 8**
(2026-10-21, ADR 0086: `En_Bigokuta`, `En_Vali`, `En_Tp` whole, one-points 3070, 3080, 3090; the runs
Ruto onto room 3's blue switch and Ruto to the Sapphire and Big Octo's arrival, room 3's way up
its spout; the presets `jabu-sapphire`, `jabu-big-octo`; every actor Jabu-Jabu places ported)
**done** (hand-tested 2026-10-22). **Milestone 9** (2026-10-23, ADR 0087: the ocarina's recording
and the box's scarecrow and frog modes, BACKLOG #47; `En_Fr`, `En_Kakasi`, `En_Kakasi3`,
`En_Kakasi2` whole, one-points 4110, 2260, 2270, 2280, pack format 32; the runs the frogs' six songs
and song game and Bonooru's recording; the presets `ocarina-c`, `frogs-choir`, `scarecrow-song`,
`scarecrow-pierre`) **done** (the hand test deferred); every actor the phase places is ported.
**Phase 11 is done** (2026-10-23) with its exit review (GAME-10's last section): Phase 12 starts
from `ocarina-of-time`. Found: in MQ neither Ruto nor Big Octo is on the way to
Barinade; the silver scale is required for Ruto's letter; door 14 is room 14's way out, not in
(room 1's spout across it drops only with the boomerang's chest).

## Phase 12: the Temple of Time, the Chamber of the Sages and adult Link (GAME-11)

**Goal:** from `ocarina-of-time`, the Door of Time opened with the Song of Time, the Master Sword
pulled, Ganondorf, the Chamber of the Sages (Rauru, the Light Medallion), adult Link back in the
Temple and Sheik's awakening, out through the ruined market; then time travel both ways (Sheik's
Prelude of Light, the sword put back and pulled again, the Prelude's warp). Exit: adult Link out
of the Temple. See [GAME-11](GAME-11-the-temple-of-time-and-adult-link.md)'s survey.
**Surveyed and proposed** (2026-10-23): three milestones. 1 is adult Link and the time machine:
the age switch (`Inventory_SwapAgeEquipment`, `linkAgeOnLoad`), `Bg_Toki_Swd`, `Door_Toki`,
Player's time travel and the adult's draw details. 2 is the Door of Time to the Chamber and out,
the exit: `En_Rl`, `SDC_CHAMBER_OF_THE_SAGES`, `En_Xc`'s awakening, `Demo_6K`'s light balls.
3 is the Prelude, the sword put back and the warp (BACKLOG #26). About 2,300 lines of C; the
spikes' Player and the import already cover most of adult Link. Decided 2026-10-23 (every
recommendation: the Prelude and the way back from a debug start with `EVENTCHKINF_48`, the
equipment page left for Phase 20, `Bg_Spot00_Break` in, `En_Xc` and `Demo_6K` split by type, the
coverage corrections with milestone 1).
**Milestone 1** (2026-10-23, ADR 0088: the age switch, `linkAgeOnLoad` and
`Inventory_SwapAgeEquipment` whole; `Bg_Toki_Swd`, `Door_Toki`; Player's adult pedestal, start
mode 1 and `TimeTravelEnd`, cutscene modes 18 and 30; the adult's gauntlets, boots, tunic colour
and left-hand overrides, pack format 33; the runs the pull and the sword put back on the presets
`master-sword` and `master-sword-adult`; milestone 2's `En_Rl`, `Demo_6K` 14 to 19 and `En_Xc` 9,
1 and the pedestal block merged early; BACKLOG #37 and #57 closed) **done** (hand-tested
2026-10-23). Found: the first adult's C buttons hold bombs and the Ocarina of Time (the survey said
empty).
**Milestone 2** (2026-10-23, ADR 0089: `SDC_CHAMBER_OF_THE_SAGES`, `chamberCutsceneNum` and
`Bg_Spot00_Break` whole; the story's five runs chained by the presets `door-of-time`,
`light-medallion` and `adult-awake`: the Door of Time, the first pull to the Chamber of the Sages,
Sheik's awakening, out of the Temple (the exit) and on through the ruined market round its ReDeads
to the field; milestone 3's `En_Xc` type 2 merged early, pack format 34) **done** (hand-tested
2026-10-23). Phase 12's coverage row is complete. **`adult-awake` is Phase 13's start.**
**Milestone 3** (2026-10-24, ADR 0090: Player's draw switch, `ActorImpl::has_draw`, Link gone in
a warp song's white-out and arrival, BACKLOG #26 closed; the runs Sheik's Prelude of Light (from
the debug preset `prelude`: the Forest Temple's `EVENTCHKINF_48` is Phase 14's), the sword put back
and pulled again, and the Prelude's warp, on the presets `prelude-learned` and `child-again`)
**done** (hand-tested 2026-10-24). **Phase 12 is done** (2026-10-24) with its exit review
(GAME-11's last section): Phase 13 starts from `adult-awake`.

## Phase 13: Epona, Ingo's races and the adult Hyrule Field (GAME-12)

**Goal:** from `adult-awake`, Lon Lon Ranch as an adult: Ingo's rental, Epona called with Epona's
Song and ridden, the two races and the bet, Epona won, the gate shut and the jump out, riding in
Hyrule Field; then the ranch after Epona and the field's Poes. Exit: Epona won. See
[GAME-12](GAME-12-epona-and-the-adult-field.md)'s survey. **Surveyed and proposed** (2026-10-24):
four milestones (riding Epona; Ingo, the rental and the races; Epona won and out, the exit; the
ranch after and the field's Poes). About 7,100 lines of C, `En_Horse` (3,868) and Player's riding
the most of it. Decided 2026-10-24 (every recommendation: no scripted race, the race's presets
with rupees, `En_Po_Field` in, a debug `epona` preset for the field).

**Milestone 1, riding Epona, is done** (2026-10-25, ADR 0091; hand-tested, committed and pushed): `En_Horse` whole but Phase 18's parts, `Bg_Umajump`, `z_horse.c`, Player's riding
whole, the horse camera (`Camera_Normal3`), the carrots; the runs `ride-epona` (the corral: her
song, on her, a fence jumped, off her) and `field-ride` (the debug `epona`: up the ramp and into the
ranch on her). BACKLOG #51 closed.

**Milestone 2, Ingo, the rental and the races, is done** (2026-10-25, ADR 0092; hand-tested,
committed and pushed): `En_In`'s adult whole, `Bg_Ingate` and `En_Horse_Game_Check`
whole; the runs `ingo-rental` (the rental to Epona mounted), `ingo-race` (the race taken from
horseback, to its start), `ingo-lost`, `ingo-won`, and the presets `ingo-rental`, `ingo-race-lost`,
`ingo-first-win`.

**Milestone 3, Epona won and out, is done** (2026-10-26, ADR 0093; hand-tested, committed and
pushed): **Phase 13's exit is met.** The run `epona-won` (the second race won: Ingo's
gate scene, a spurred gallop over the shut gate, the field's leap; Epona Link's) ends as the
preset `epona-won`, **Phase 14's start**.

**Milestone 4, the ranch after Epona and the field's Poes, is done** (2026-10-26, ADR 0095;
hand-tested, committed and pushed): `En_Ma3` (Malon's obstacle course: the offer, the results,
the high score, the cow), `En_Ma2`'s adult (Epona's Song played to her by night) and
`En_Po_Field` (the field's Big Poes, by day or night) whole; the message box's timers and high scores; the runs
`malon-course` (to the course's start), `malon-record` (on its debug preset), `malon-song` and
`big-poe` (it appears and flees). The laps, the fight and the bottle are the hand test's.
**Phase 13 is done** (2026-10-26) with its exit review (GAME-12's last section): Phase 14
starts from `epona-won`.

## Phase 14: The Forest Temple (GAME-13)

**Goal:** from `epona-won`, the hookshot from Dampé's race, the Lost Woods past Mido and the
meadow's Moblins, Sheik's Minuet of Forest, and the Forest Temple: the bow, the Poe sisters, the
twisted corridors, Phantom Ganon. Exit: the Forest Medallion. See
[GAME-13](GAME-13-the-forest-temple.md)'s survey. **Surveyed and proposed** (2026-10-27): six
milestones (the hookshot and Dampé's race; the Lost Woods, the meadow and the Minuet; the
temple's front and the bow; the courtyards and the twisted corridors; the Poe sisters; Phantom
Ganon and the Forest Medallion, the exit). About 16,900 lines of C, the temple about 13,500 of
them: the largest phase yet. Decided 2026-10-27 (every recommendation: `En_Hs` and `En_Mm2` to
Phase 20, `En_Honotrap` whole, Phantom Ganon's decay as the C, the race and the maze the hand
test's, the adult bean lift later).

**Milestone 1, the hookshot and Dampé's race, is done** (2026-10-27, ADR 0096; hand-tested,
committed and pushed): `Arms_Hook` and Player's hookshot whole (the flight, its camera `Camera_Special5`, the
reticle), `En_Po_Relay` (Dampé's race) and `En_Honotrap` whole; the runs `dampe-race` (to the
first door) and `hookshot` (Goron City's stone post), and the preset `hookshot`.

## Cross-cutting debts

Pick these up when a milestone touches them, or as filler:
- **Camera:** the quakes are in (GAME-05 milestone 4a, ADR 0039), ~~but the camera's roll isn't
  drawn (the renderer keeps Y up)~~ (the up modelled in GAME-09 milestone 4, ADR 0075); the shake
  isn't applied to the prerendered backgrounds and the skybox.
- **Rendering:** actor culling (`Actor_CullingCheck` / `Actor_CullingVolumeTest` in `Actor_DrawAll`; Hyrule Field's spawned trees and Peahats wait on it, BACKLOG #32), ~~shadows (`ActorShadow_Draw*`)~~ (GAME-09 milestone 6's hand test: `oot_game::shadow`), ~~the exit's circle wipe (`TransitionCircle`)~~ (GAME-07 milestone 2). The effect systems are in (GAME-05 milestone 3a, ADR 0033; four more overlays in 3b, ADR 0037) with the overlays their callers needed (the bushes', rocks' and crates' pieces, `Effect_Ss_Kakera`, in 4b); ~~the sword's and the Skulltula's trails (`EffectBlure`)~~ (GAME-09 milestone 3); next to want: the Gold Skulltula's and its token's shine (`func_8002EBCC`'s and `func_8002ED80`'s look-at for texgen: lit from the camera's view for now).
- **World:** ~~time passing, day/night~~ (GAME-06 milestone 3); weather.
- **HUD and messages:** the minimap (its data is in since GAME-05 milestone 4a: `MapState`, `table/map`); the item icons and backgrounds in text; ~~the ocarina modes~~ (GAME-07 milestone 3).
- **Randomness:** the effects' `Rand` calls in the C's order, where effects are ported. Unchecked: the order of several `Rand` calls in one call's arguments (`En_Goma`'s hatch debris and its hurt colours assume IDO evaluates them left to right); check it against the disassembly.
- **Tools:** the `oot_extract::text` refactor onto `oot_import::text`.
- **Faithfulness checks:** side-by-side captures against Project64 are left for a later polish pass (the user's call, 2026-10-01): milestones end with tests from the C, goldens and a hand-test list instead.

## The whole game (outline, 2026-10-13)

The rest of the game in the story's order, as phases. This is the order and the scope, not the
split: each phase is surveyed and split into milestones when it starts (below), and the survey
corrects the outline. [COVERAGE.md](COVERAGE.md) lists the actors each phase's scenes place, and
which are ported; its phases come from `scripts/coverage.py`'s `SCENES` table, which changes with
this outline. The dungeons are Master Quest's.

| Phase | Places | Dungeon and boss | What it brings | Exit |
|---|---|---|---|---|
| 8 (done) | Castle Town, Hyrule Castle, Kakariko, the graveyard | | Milestones 5 and 6 above: lifting and throwing, the cuccos, Dampé, the royal tomb | The Sun's Song learned |
| 9 (done) | The Lost Woods, the Sacred Forest Meadow, Lon Lon Ranch | | Saria in her meadow and Saria's Song (Darunia needs it), the Lost Woods' maze and the Skull Kids' memory game (#47), Malon and Epona's Song, the child's Epona, Talon's ranch (#44) | Saria's Song learned |
| 10 | Death Mountain Trail and summit, Goron City | Dodongo's Cavern; King Dodongo | Bombs (`En_Bom`, BACKLOG #33) and bomb flowers, the Goron's Bracelet, Darunia's dance, the summit's Great Fairy (the magic meter) | The Goron's Ruby |
| 11 | Zora's River, Zora's Domain, Zora's Fountain, Lake Hylia | Jabu-Jabu's Belly; Barinade | Bottles (the fish, Ruto's letter), diving (the silver scale), Ruto carried, the boomerang, the frogs and the scarecrow (#47) | The Zora's Sapphire, then the Ocarina of Time at the moat |
| 12 | The Temple of Time, the market's ruins, the Chamber of the Sages | | The Door of Time (#37), the Master Sword, adult Link throughout (models, items, the save's age), time travel back and forth, the Prelude of Light, the warp songs | Adult Link out of the Temple |
| 13 | Lon Lon Ranch and Hyrule Field as an adult | | Riding Epona (`En_Horse` whole: the horse camera, jumps), Ingo's races | Epona won |
| 14 | Kokiri Forest, the Lost Woods and the meadow as an adult; the graveyard (Dampé's race) | The Forest Temple; Phantom Ganon | The hookshot, the bow, the Minuet of Forest | The Forest Medallion |
| 15 | Death Mountain Crater, Goron City as an adult | The Fire Temple; Volvagia | The Goron Tunic and heat, the Megaton Hammer, the Bolero of Fire | The Fire Medallion |
| 16 | Zora's Domain frozen, Lake Hylia as an adult | The Ice Cavern; the Water Temple; Morpha | Iron boots, the Zora Tunic, blue fire, the longshot, the Serenade of Water, the lake's water | The Water Medallion |
| 17 | Kakariko burning and as an adult | The Bottom of the Well (as a child); the Shadow Temple; Bongo Bongo | The Lens of Truth, hover boots, the Nocturne of Shadow | The Shadow Medallion |
| 18 | Gerudo Valley, Gerudo's Fortress, the Haunted Wasteland, the Desert Colossus | The Gerudo Training Ground; the Spirit Temple (child and adult halves); Twinrova | The silver gauntlets, the mirror shield, the Requiem of Spirit, the carpenters freed, horseback archery | The Spirit Medallion |
| 19 | Outside Ganon's Castle | Ganon's Castle (the six trials), the tower; Ganondorf, Ganon | The light arrows, the golden gauntlets, the collapse's escape, the ending and the credits | The credits |
| 20 | Everywhere | | Breadth: the shops, the games (the shooting galleries, bombchu bowling, the treasure chest game, fishing, the diving game, the horse race), the Gold Skulltulas and their house, the trade sequences (the masks, Biggoron's sword), the Great Fairies' spells (magic, #49), the grottos, heart pieces, the scarecrows' songs, the pause menu in full (#50: the quest page, the world map), the minimap, the title screen and the file select | Everything optional reachable |

**Systems that cut across the phases** (each comes with the first phase that needs it, then grows):
- **Link's items:** bombs (10), the boomerang and bottles (11), the Master Sword and adult items (12),
  hookshot and bow (14), hammer (15), iron boots and longshot (16), lens and hover boots (17),
  gauntlets and mirror shield (18), light arrows (19), magic and the spells (10, then 20).
- **Adult Link** (12): adult Link already moves and draws (the spikes' Link was adult; his
  meshes are the pack's `player/adult/...`), but every route so far is a child's. Adult Link's
  items, tunics and the save's age switch come with Phase 12.
- **Epona** (9 as a foal, 13 ridden): `En_Horse` is one of the largest actors; its camera and
  Player's riding actions come with it.
- **Bosses and their rooms** (Phases 10, 11 and 14 to 19, one each, two in 19): each its own milestone, as Gohma was.
- **The pause menu and the HUD:** the equipment and quest pages, the world map, the minimap, the
  title and file select (Phase 20, earlier where a phase needs a part).
- **The scene draw configs** (BACKLOG #46 and its kind): each with the phase whose scenes use it.

## The game editor track

Alongside the phases, at the user's pace: the editor grows from the overworld builder into the
game editor (ADR 0062). The game's phases feed it: every actor ported is one the editor can place.
In stages:

| Stage | What | Where it's planned | State |
|---|---|---|---|
| E1 | The overworld builder: regions, kits and themes, paths, stairs, ceilings | ADR 0035, ADR 0036, [OVERWORLD-EDITOR-IDEAS.md](OVERWORLD-EDITOR-IDEAS.md) | Built (2026-10-05 to 10-11) |
| E2 | Play in the real game: a level is a scene of its own, entered by `Play_Init` | [EDITOR-PLAY-IN-THE-GAME.md](EDITOR-PLAY-IN-THE-GAME.md) steps 1 to 4 | Built (2026-10-11) |
| E3 | The actors tab: any ported actor placed, its params and flags as named fields (from each actor's schema, ADR 0066), its objects added to the room, flags handed out | [EDITOR-ACTORS.md](EDITOR-ACTORS.md), ADR 0077, ADR 0078, ADR 0079 | First built 2026-10-17 (the user's call to start it): the tool, palette, inspector, layers, object needs, Play. 2026-10-19: the actors as the game draws them (captured from their own draw code by `oot capture-actors`, drawn by the game's renderer in 3D, thumbnails, outlines in the plan, invisible actors' areas). 2026-10-19: the texts (ADR 0079: each actor's lines in its schema, the Dialogue section drawing them with the game's message box, a level's edited lines played in its own scene). Next the tuning pass, paths, the flags board |
| E4 | Exits and rooms: exits between levels and the game's scenes, transition actors, levels of several rooms | EDITOR-PLAY-IN-THE-GAME.md, "outside this plan" | Planned |
| E5 | Inside and down: seeing inside, storeys, dungeon mode (rooms and portals, the generated map), flags wiring, the completability check | [EDITOR-ROOFS-STOREYS-DUNGEONS.md](EDITOR-ROOFS-STOREYS-DUNGEONS.md) steps 2 to 5, in ten slices | Step 1 built; slice 1 (storeys) built 2026-10-25 |
| E6 | A scene's data: paths, cameras (bg cameras by polygon), water boxes, lights and the environment, the sky, the clock, music and ambience, the restrictions | To plan | |
| E7 | Content: texts (the message table), cutscenes (a script's timeline), chests, drops and shops | To plan | |
| E8 | The game's own scenes in the editor: Kokiri Forest (or any scene) opened as a document (its rooms as fixed meshes, with everything above editable), changed and played | To plan; the step that makes "our own version of the game" | |
| E9 | Sharing: a project of many scenes as a mod pack, glTF in, the decomp-free import | ADR 0035 | |

E6 to E9 get their plans (and ADRs) when the track reaches them; each needs the pack's records for
its kind of data to be complete, which the phases make so.

glTF out, E9's "glTF in" the other way: `ootx export-glb` writes the game's characters by name as
skinned, T-posed .glb files into `out\glb\characters` (ADR 0068, built 2026-10-14).

## How a phase starts and how a milestone is done

**A phase starts with a survey:** the C functions and their line counts, what's already ported,
the scenes and actors the story's route meets (COVERAGE.md's row for the phase is the start),
the systems it needs; then a report, a proposed split into milestones, and the user's calls on
anything that's theirs (asked, not assumed). The phase doc (`docs/GAME-NN-<name>.md`) records it.

**A milestone is done when:**
1. Its functions are ported whole (or as far as the user agreed), with the decomp's names, every
   constant cited and faithful bugs marked `@bug (game)`; new actors registered in
   `oot_actors/src/lib.rs`'s four places and matching the actor table.
2. Each new actor's module docs end with its **Params** and **Flags** paragraphs (ADR 0062), and
   the module has its **schema** (`pub const SCHEMA`, the profile's `schema`; ADR 0066): every
   field checked against the decomp, the code reading its params through the fields, and
   `oot_actors --test schemas` passing (the code, the flag calls, the text ids its code uses
   as the schema's `texts`, ADR 0079, and every placement in the game's data).
3. Its tests have expectations from the C, and a short scripted exit run is a golden (a trace and
   screenshots), the same over two runs. Every changed golden hash is proven by taking its cause
   out, and logged in `golden/README.md` field by field, with the outputs saved before recording.
4. `cargo test --release --workspace` and `cargo test -p layering` pass; the release build has no
   warnings.
5. Scripts `test-`, `game-` and `sandbox-<name>.bat` (CRLF) are in `scripts/run/menu.bat` with
   rows in `scripts/run/README.md`.
6. The docs: the phase doc's milestone section (the answer, scripts, what was built, results,
   decisions, known gaps, fixes, how to check, by hand), ADRs for its decisions, BACKLOG rows for
   what's left, ROADMAP's status, and COVERAGE.md refreshed (`scripts\run\coverage.bat`).
7. It ends with the check commands and a concrete hand-test list (keys and places, no emulator
   side-by-side), and is committed and pushed only when the user asks.
## Beyond the game

When the outline above is played through: the game editor's later stages (E6 to E9) for making
our own version, and the decomp-free import for sharing it. The custom-levels track itself is
under "The game editor track" above.
