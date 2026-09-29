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

## Where things stand (2026-09-28)

**Done:** Phases 0 to 3 (GAME-01 and GAME-02). Kokiri Forest plays with:
- collisions, props, climbing, Z-targeting, doors and the prerendered interiors;
- talking, the message box, drops and a minimal HUD;
- the Deku Tree's mouth, opened by a save flag.

GAME-03 milestones 1 and 2 are done too:
- the inventory, `Item_Give`, the get-item flow, the chests, and Link's equipment from the save;
- the crawl and the crawlspace's camera, the scene paths, the training area's boulder and wonder
  items, and Link's knockdown.

Two headless scripted runs start in Link's bed (`oot_actors --test playthrough`): into the Deku
Tree (on a flag preset), and on a new save to the Kokiri Sword's chest, opened.

**Shortcuts the next phases have to undo:**
- ~~A new save gives child Link the Kokiri Sword and a shield on B.~~ Undone in GAME-03 milestone 1: a new save is `Sram_InitNewSave`'s, and the presets own and wear them (ADR 0019).
- ~~The inventory isn't kept.~~ Undone in GAME-03 milestone 1.
- The pause menu isn't ported: Start stands in for its equipping (ADR 0019).
- Unported actors are placeholders. Among them: Navi, Mido, Saria and the shopkeeper.
- There are no cutscenes: the importer skips all 73. The Deku Tree's mouth opens only with the `deku-tree-open` save preset, which the playthrough starts on.
- No audio at all.

## Phase 3, the end: GAME-02 milestone 4 (done)

Done as planned, with one correction: the open mouth is `EVENTCHKINF_05`, not `EVENTCHKINF_07` (see [GAME-02](GAME-02-kokiri-forest.md) milestone 4 and ADR 0018). The plan as it was:

**Goal:** close the Kokiri Forest slice with its exit test from the plan: a headless scripted playthrough as a regression test.

**What the C needs.** The Deku Tree's mouth is cutscene-driven (`z_bg_treemouth.c`):
- Talking to the tree starts a cutscene (`D_808BCE20`, setting `EVENTCHKINF_0C` and `EVENTCHKINF_05`).
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
  7. into the mouth: the exit to `ENTR_YDAN_0`, ending in the Deku Tree scene.

  The same inputs also write a JSON trace, added to the goldens.
- **The interactive checks** listed at the end of GAME-02, against Project64, before recording the playthrough.

**Exit:** the playthrough passes headless; GAME-02 is marked complete; the architecture plan's Phase 3 is marked done.

## Phase 4: the road to the Deku Tree (GAME-03)

**Goal:** a new save plays Kokiri Forest the game's way, up to entering the Deku Tree:
- wake up;
- get the Kokiri Sword from its chest and the Deku Shield from the shop;
- Mido steps aside;
- the Deku Tree's cutscene opens his mouth.

1. **The inventory and getting items** (done: see [GAME-03](GAME-03-road-to-deku-tree.md) milestone 1 and ADR 0019).
   - `SaveContext`'s inventory: items, equipment, upgrades, quest items, ammo. A new save as `Sram_InitNewSave` makes it.
   - `Item_Give` in full for what Kokiri Forest gives.
   - Player's get-item flow:
     - `func_8002F434` / `func_8002F554` (later decomps call the first `Actor_OfferGetItem`);
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
3. **Mido and the shop.**
   - `En_Md`: blocking the path, his talk states, stepping aside with the sword and shield (`EVENTCHKINF` flags).
   - `En_Ossan`, the Kokiri shop only:
     - the browsing camera (`PIVOT_SHOP_BROWSING` is ported; the shopkeeper switches the viewpoint);
     - choosing, buying, and the rupee check;
     - the shelf items (`En_GirlA`).
   - **Exit:** buy the Deku Shield with the rupees from the bushes; Mido lets Link through.
4. **Cutscenes, first part** (`z_demo.c`; an ADR).
   - Import the cutscene scripts.
   - `csCtx`: the camera commands, Player's and the actors' cues (`npcActions`), text commands through the message box, transitions and terminators.
   - `Cutscene_HandleEntranceTriggers` and `cutsceneTrigger`.
   - Player's cutscene modes (`func_8002DF54`), which also completes `En_Wonder_Talk2`'s forced texts.
   - **Exit:** talking to the Deku Tree plays `D_808BCE20` to the end, yes plays `D_808BD520`, and his mouth opens (`EVENTCHKINF_05`); the scripted run from GAME-02 milestone 4 opens it the game's way, without the preset.
5. **Navi.**
   - `En_Elf` as Link's fairy: following, the target reticle's `naviRefPos`, C-Up and her text.
   - The Kokiri children's fairies.
   - The game's opening, if the cutscene system covers it: Navi waking Link (`ENTR_LINK_HOME` with its cutscene layer).
   - **Exit:** a new save starts as the game does, and C-Up talks to Navi.

**Phase exit:** a headless run from a new save to the Deku Tree scene, the game's way, replacing GAME-02 milestone 4's flag preset.

## Phase 5: audio (GAME-04)

**Goal:** music and sound effects, from the ROM through the pack.

This phase doesn't depend on Phase 4, so it can go earlier, or run in between as a change of pace. `oot_extract::audio` already decodes the soundfonts, the VADPCM samples (checked bit for bit) and the sequences (a tick-accurate sequence player, to MIDI). The decomp's audio is about 18,000 lines (`audio_*.c`).

1. **The import and the synth:**
   - soundfonts, samples and sequences into the pack;
   - an `eng_audio` crate: an output device (for example `cpal`), and a synthesiser that follows `audio_synthesis.c` (ADPCM playback, envelopes, pan, reverb);
   - an ADR on how faithful the mixer is.
2. **The sequence player at runtime:**
   - `audio_seqplayer.c` driving the synth, including the game's IO ports;
   - the scene's music (`Environment_PlaySceneSequence`, `func_800F5550` and the rest of `code_800EC960.c`), the ambience (`Audio_PlayNatureAmbienceSequence`).
   - **Exit:** Kokiri Forest's music plays and loops like the game.
3. **Sound effects:**
   - `Audio_PlaySfxGeneral` and the sfx channels;
   - the calls the ported code already marks as left out: Player's footsteps, sword, text blips, doors, rupees, the HUD, the low-health alarm.
   - **Exit:** a scripted run's sfx log matches the calls in the C.

## Phase 6: the Deku Tree (GAME-05)

**Decisions needed first (ADR 0003, ADR 0004):**
- **Master Quest or vanilla?** The debug ROM's Deku Tree is Master Quest. Vanilla needs a second ROM version and a decomp commit that supports it.
- **Upgrade the decomp?** The upgrade brings named functions such as `Player_Action_*`, and needs a name map for the existing citations.

Both should be settled at the start of the phase, before any dungeon work.

1. **Damage and health:**
   - Player taking damage: knockback, invincibility frames, `Health_ChangeBy`. *(Mostly done in GAME-03 milestone 2 for the boulder, ADR 0020: the body hit, the stagger, the knockdown, the invincibility timer, the fall damage. Left: kinds 3 and 4 (frozen, shocked), the hit while swimming, burning, the red flash.)*
   - death and game over (`Play_TriggerVoidOut` exists);
   - enemy damage tables (`CollisionCheck_ApplyDamage`, `DamageTable`).
   - **Exit:** the dummy and a Deku Baba hit Link.
2. **The first enemies:**
   - `En_Dekubaba`, `En_St` (Skulltula), `En_Hintnuts` / `En_Dekunuts` (Deku Scrubs);
   - enemy targeting and `Camera_Battle1`;
   - drops on death;
   - the effects they need (`EffectSs`: dust, hit sparks, the death flame).
3. **Dungeon mechanics:**
   - `Door_Shutter` and small keys;
   - switches, torches (lighting Deku sticks), webs to burn or fall through;
   - the map and compass in the pause data;
   - `Bg_Ydan_*` (`Bg_Ydan_Hasi` is ported from the spikes).
4. **Items in use:**
   - Deku sticks and nuts, the Fairy Slingshot (`EnArrow` for seeds);
   - the C buttons in full;
   - a minimal pause menu for equipping (`z_kaleido_scope` is about 7,700 lines, so only the item screen at first);
   - saving (`z_sram.c`).
5. **Gohma:**
   - `Boss_Goma` (about 2,100 lines) and her larvae;
   - the boss room's camera and cutscenes (Phase 4's cutscene system);
   - the heart container, the blue warp out.
   - **Exit:** a scripted run through the Deku Tree to Gohma's defeat.

## Cross-cutting debts

Pick these up when a milestone touches them, or as filler:
- **Rendering:** actor culling (`func_800314B0` / `func_800314D4` in `Actor_DrawAll`), shadows (`ActorShadow_Draw*`), the exit's circle wipe (`TransitionCircle`), the effect systems (`EffectSs`, `Effect`). The first effects to want: the bushes' and rocks' flying pieces (`EffectSsKakera` from `EnKusa_SpawnFragments` and `EnIshi`), and dust.
- **World:** time passing (`Environment_Update`'s clock), day/night, weather.
- **HUD and messages:** the minimap; the item icons and backgrounds in text; the ocarina modes.
- **Randomness:** the effects' `Rand` calls in the C's order, where effects are ported.
- **Tools:** the `oot_extract::text` refactor onto `oot_import::text`.
- **Faithfulness checks:** side-by-side captures against Project64 for each milestone's visuals, kept as a checklist in the phase doc.

## Beyond

The plan's later items, roughly in order:
- Hyrule Field and Kakariko (Epona waits for her own phase);
- the other dungeons;
- the full message system (choices with items, the ocarina);
- the pause menu in full.

Then breadth across the rest of the 427 actor overlays and 36 effect overlays, in the order scenes need them. And a parallel track for custom levels: glTF to a mod pack, and the decomp-free import for sharing.
