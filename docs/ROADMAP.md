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

## Where things stand (2026-10-02)

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
pack format 24. Next: 5b-2, the dungeon map page and the game over drawn.

**Priorities (2026-10-01):** the cutscenes are finished properly now, as their own phase, so later
work doesn't have to think about them (the user decided against skipping them). This replaces
the 2026-09-30 deferral: the scripted runs play the cutscenes.

**Shortcuts the next phases have to undo:**
- ~~A new save gives child Link the Kokiri Sword and a shield on B.~~ Undone in GAME-03 milestone 1: a new save is `Sram_InitNewSave`'s, and the presets own and wear them (ADR 0019).
- ~~The inventory isn't kept.~~ Undone in GAME-03 milestone 1.
- The pause menu's frame and item page are ported (ADR 0047); the equipment, quest and map pages show only their backgrounds (their contents logged), and the equipment page's equipping is a stand-in run as the menu closes: every owned piece of a type with nothing worn goes on (ADR 0019, 0047). The save prompt and the debug inventory editor log. The game over screens run undrawn (5b-2), and "Continue? No" respawns instead of going to the title screen (ADR 0032).
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

## Phase 6: the Deku Tree (GAME-05)

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
     - **5b-2, the dungeon map page and the game over:** `z_kaleido_map.c`'s dungeon map
       whole with `z_lmap_mark.c`'s marks; the game over screens drawn through
       `KaleidoScope_Draw`.
   - **5c, saving:** `z_sram.c`'s save and load whole, the slots on disk in the C's layout, the
     pause menu's save prompt, the game over's `Sram_WriteSave`, and a stand-in for the file
     select's load.
6. **Gohma:**
   - `Boss_Goma` (about 2,100 lines); her eggs and larvae (`En_Goma`) are pulled forward to 3b;
   - the boss room's camera and cutscenes (Phase 4's cutscene system);
   - the heart container, the blue warp out.
   - **Exit:** a scripted run through the Deku Tree to Gohma's defeat.

## Cross-cutting debts

Pick these up when a milestone touches them, or as filler:
- **Camera:** the quakes are in (GAME-05 milestone 4a, ADR 0039), but the camera's roll isn't
  drawn (the renderer keeps Y up), and the shake isn't applied to the prerendered backgrounds
  and the skybox.
- **Rendering:** actor culling (`Actor_CullingCheck` / `Actor_CullingVolumeTest` in `Actor_DrawAll`), shadows (`ActorShadow_Draw*`), the exit's circle wipe (`TransitionCircle`). The effect systems are in (GAME-05 milestone 3a, ADR 0033; four more overlays in 3b, ADR 0037) with the overlays their callers needed (the bushes', rocks' and crates' pieces, `Effect_Ss_Kakera`, in 4b); next to want: the sword's and the Skulltula's trails (`EffectBlure`); the Gold Skulltula's and its token's shine (`func_8002EBCC`'s and `func_8002ED80`'s look-at for texgen: lit from the camera's view for now).
- **World:** time passing (`Environment_Update`'s clock), day/night, weather.
- **HUD and messages:** the minimap (its data is in since GAME-05 milestone 4a: `MapState`, `table/map`); the item icons and backgrounds in text; the ocarina modes.
- **Randomness:** the effects' `Rand` calls in the C's order, where effects are ported. Unchecked: the order of several `Rand` calls in one call's arguments (`En_Goma`'s hatch debris and its hurt colours assume IDO evaluates them left to right); check it against the disassembly.
- **Tools:** the `oot_extract::text` refactor onto `oot_import::text`.
- **Faithfulness checks:** side-by-side captures against Project64 for each milestone's visuals, kept as a checklist in the phase doc.

## Beyond

The plan's later items, roughly in order:
- Hyrule Field and Kakariko (Epona waits for her own phase);
- the other dungeons;
- the full message system (choices with items, the ocarina);
- the pause menu in full.

Then breadth across the rest of the 427 actor overlays and 36 effect overlays, in the order scenes need them. And a parallel track for custom levels: glTF to a mod pack, and the decomp-free import for sharing.

The custom-levels track has started (ADR 0035): the overworld builder and editor (`crates/tools/overworld`,
`crates/tools/overworld_editor`) make Kokiri Forest-style levels, and `oot_sandbox --level` plays them with child Link
in the spikes' view. Next on it: the level as a mod pack entered by `Play_Init`, actors placed in the editor, hookshot
targets, exits, and rooms for big levels.
