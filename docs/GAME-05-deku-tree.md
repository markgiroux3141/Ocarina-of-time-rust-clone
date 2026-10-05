# GAME-05: the Deku Tree

**Goal:** Phase 6 of the [roadmap](ROADMAP.md): the first dungeon, the Deku Tree, as the debug
ROM has it (Master Quest), up to Gohma's defeat. Decided in
[ADR 0028](adr/0028-phase-6-master-quest-and-the-decomp-upgrade.md): gc-eu-mq-dbg stays the only
ROM, and the decomp is upgraded first, before any dungeon work.

| # | Milestone | Status |
|---|---|---|
| 1 | The decomp upgrade: an address-based name map from `2f4c25d`'s names to the new commit's, the citations migrated by it, the importer on the new layout, the pack's record names renamed (a format bump); every test passes and the goldens are the same bytes | done |
| 2 | Damage and health: Player taking damage (kinds 3 and 4, the hit while swimming, burning, the red flash), death and game over, the enemies' damage tables (`CollisionCheck_ApplyDamage`, `DamageTable`) | done |
| 3 | The first enemies: `En_Dekubaba` (ported in milestone 2 but its effects), `En_St` (Skulltula), `En_Hintnuts` / `En_Dekunuts` (Deku Scrubs); enemy targeting and `Camera_Battle1`; drops on death; the effects they need (`EffectSs`) | |
| 4 | Dungeon mechanics: `Door_Shutter` and small keys; switches, torches, webs; the map and compass; the `Bg_Ydan_*` actors | |
| 5 | Items in use: Deku sticks and nuts, the Fairy Slingshot; the C buttons in full; a minimal pause menu for equipping; saving (`z_sram.c`) | |
| 6 | Gohma: `Boss_Goma` and her larvae; the boss room's camera and cutscenes; the heart container and the blue warp. **Exit:** a scripted run through the Deku Tree to Gohma's defeat | |

The working rules are the same as for the earlier phases:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack, and only the importer reads the decomp;
- ports go function by function with the decomp's names, every constant is cited, and faithful
  bugs are marked `@bug (game)`;
- test expectations come from the C;
- a change in a run's trace is a golden change to explain.

From milestone 1 on, the decomp is zeldaret/oot main at `52a510f`
([ADR 0031](adr/0031-decomp-main.md)), and new ports cite its names. Decisions are in
[docs/adr/](adr/README.md) (0031 on).

## Milestone 1: the decomp upgrade

**Answer:** done. The repo now cites the decomp at zeldaret/oot main, commit `52a510f`
(2026-09-30), instead of `2f4c25d` (2022-10-04), and the importer reads it. Nothing behaves
differently:
- every test passes;
- every render is the same bytes;
- the traces are the old ones' bytes once their C names are renamed through the map.

The pack is format 16, in `out/data13`.

### What was built

**The two builds.** Both commits build the user's ROM (gc-eu-mq-dbg) byte for byte, so the same
function or variable sits at the same address in both builds.
- Each decomp is built in Docker, with its own Dockerfile, in a volume. The checkouts are only
  read: the old one (`D:/OOT Modding/OTT decomp/z64oot`) stays as it is.
- The build scripts are `scripts/decomp/build_old.sh` and `build_new.sh`, run by
  `scripts\run\name-map.bat`.

**The name map** (`docs/name-map`, [its README](name-map/README.md)). `scripts/name_map.py
build` reads the two ELFs and the two checkouts' C and writes the names that changed: 36,304
rows, names and addresses only.
- **Functions and data** by address: each ELF's `.symtab`, plus the merged ECOFF `.mdebug`,
  where IDO keeps the `static` functions and data. The assets' symbols are paired per output
  section (the ROM file), since segmented addresses repeat.
- **Struct fields** by offset, from the `/* 0xNN */` comments. Nested structs are flattened, and a
  small vector's members take their parent's name. Renamed structs are paired by slot and by
  their members' names.
- **Enum members** by value within their enum. A table-generated enum (`NA_BGM` from
  `sequence_table.h`, the actor and object ids) is paired by row. An old name counts as unchanged
  only if main has it with the same value: `CS_CMD_CAM_EYE` exists on main with another value.
- **`#define`s** by family prefix and value, in the headers the old header's contents moved to.
  A renamed family (`TOUCH_` to `ATELEM_`, `BUMP_` to `ACELEM_`) is paired by majority.
- **Files** by the symbols they define. A file split in two is listed with its successors, and
  its citations take the main one when it has at least 40% of them.
- **Partly renamed names.** A name renamed in one file and kept in another (`sTunicColors` in
  `z_player.c`, `sShirtColors` elsewhere) gets a row to itself as well, so it is only renamed
  where the file is known.
- **The rest by hand.** 162 successors that can't be paired mechanically (macros, anonymous
  enums, types whose fields changed too) are checked by hand in `manual.tsv`, each with its
  evidence.
- **The unpaired.** 639 names, and one pairing with two candidates, are in `unpaired.tsv` and
  were reviewed.

`python scripts/name_map.py find NAME` looks a name up either way.

**The citations.** A script driven by the map (kept outside the repo, its diff reviewed before
it was applied) renamed about 7,500 citations in 216 files:

| Kind | Names | Citations |
|---|---|---|
| Functions in comments, strings and docs | 457 | 1,700 |
| Data | 173 | 536 |
| Constants | 258 | 773 |
| Files | 63 | 366 |
| Types | 31 | 94 |
| Fields | 94 | 230 |
| Rust identifiers named after C functions and statics | 385 | 2,321 |
| Rust constants | 325 | 1,478 |
| Rust fields (per struct, then the compiler's errors for their uses) | 22 | 48 |

- **Naming rules.**
  - Comments, strings and docs take the new C names verbatim.
  - Rust functions take the new name in snake case, without the module's own prefix (`Player_`
    in `player.rs`, `Camera_`, `EnMd_`).
  - Statics take it in screaming snake case (`D_808543E0` became `S_ACTION_HANDLER_LIST1`).
  - Fields take the new member, the unknown parts kept (`unk_6AE_rot_flags`).
- **Fields per struct.** A field whose new name depends on its struct was renamed at its
  declaration, and the compiler's errors (`E0609`, `E0560`, `E0026`) gave the uses.
- **Hand fixes.** A second pass placed the `unk_` citations whose struct the line or the file
  names. The rest were checked by hand against main's headers ([the review](name-map/README.md)).
  So were the names main removed, rewritten to what main has instead:
  - `Audio_SeqCmdB` and `Audio_SeqCmdC` are `SEQCMD_OP_TEMPO_CMD` and `SEQCMD_OP_SETUP_CMD`;
  - `DPM_UNK` is a literal 0;
  - `<Name>_InitVars` is `<Name>_Profile`;
  - `gTatumsPerBeat` is `gTempoData.seqTicksPerBeat`;
  - `z64elf_message.h`'s `ELF_MSG_*` are `quest_hint_commands.h`'s `QUEST_HINT_*`;
  - `CsCmdBase` is `CsCmdCam`;
  - `WATERBOX_BGCAM_INDEX` is `WaterBox_GetBgCamIndex`.

**The importer on main's layout** (`oot_import`):
- **The symbol index** (`symbols.rs`) reads `baseroms/gc-eu-mq-dbg/config.yml`'s asset list. It
  applies the XMLs' `<Version Pattern>` blocks and lays out the offsets that are implied by the
  elements' sizes or relative (`.+0x`). It adds an overlay's start to its offsets and resolves
  the named palettes (`Tlut="name"`).
- **`version.rs`** (new) reads `config.yml`'s `variables` (the message and audio tables'
  addresses in `code`), its `incbins` (`aspMainData`) and `segments.csv`.
- **The C** (`csrc.rs`) is preprocessed as gc-eu-mq-dbg builds it (`VERSION_DEFINES`: its
  `#if`s, `FRAMERATE_CONST(a, b)` as `a`). A macro evaluator, `csrc::Macros`, expands object-
  and function-like macros and enums, evaluates C's integer expressions, and adds the enums
  built from `DEFINE_*` rows by row.
- **The tables** each moved to their new files and forms:
  - the boot registers;
  - Player's items (`PLAYER_IA_`, was `PLAYER_AP_`);
  - the camera's modes and settings;
  - the one-point tables (the function-local ones included);
  - the actor table (`ACTORCAT` and the flags evaluated, `<Name>_Profile`);
  - the item drops;
  - the entrances and the skyboxes (`skybox.h`, `drawType` evaluated);
  - the draw configs (`SceneDrawConfigFunc` arrays, `GRAPH_ALLOC`, `MATRIX_FINALIZE`,
    `STACK_PAD`);
  - the cutscene macros (`CS_FLOAT`, `PLAYER_CUEID`, `NA_BGM` by row);
  - Navi's texts (`QUEST_HINT_*`, their new argument order);
  - the audio tables (from the ROM at `config.yml`'s addresses; `2f4c25d` had them in
    assembly), with the sequence flags and the sound effects from their `DEFINE_*` rows, as
    gc-eu-mq-dbg builds them.
- **`oot.toml`** points at the new checkout. The pack's header records `decomp_commit`.

**Pack format 16** (`out/data13`, 65.3 MB, 12,885 records). The records named after decomp names
that changed take their new names. The cutscene layers' scripts, which `2f4c25d`'s XMLs didn't
name and the pack keyed by offset (`cutscene/spot04_scene/0xA6D0`), are keyed by main's names
(`gKokiriForestIntroNaviFlyingCs`). The XMLs' 11 extra `Scene` elements (the unused alternate
headers, `*_scene_unused`) are tallied as skipped.

**The tools.** Every `ootx` command was run with the old build on the old pack and the new build
on the new pack:
- `info`, `scan-skeletons`, `player-anims`, `player-draw`, `scan-scenes`, `dump-room`,
  `scene-info`, `pack-ls`, `sfx`, `cutscene` and `dl-dump`: the same output except main's names
  and the symbols its XMLs add.
- `ootx extract` (`oot_extract`) needed fixes for main's headers: the sequence names, the
  skyboxes, the wall flags (evaluated), and the sound effects' `DEFINE_SFX` (six arguments, the
  `#if` pairs). It now writes what it wrote from `2f4c25d`, plus main's corrections: 5,628
  textures where it wrote 4,960, and no grey-ramp palettes (68 before).

**Scripts** (`scripts\run`):
- `name-map.bat` rebuilds the decomps in Docker and regenerates `docs/name-map`;
- `decomp-check.bat` imports a loose pack and compares it with the old decomp's
  (`name_map.py compare-pack`).

### Results

**Tests.** `cargo test --workspace` (`target/game14`, `OOT_DATA_DIR=out/data13`): 319 passed, 0
failed, 1 ignored, the pack's oracle tests included. Four tests changed with main's C:
- the drop tables are 240 quantities, since main dropped `sDropQuantities`' 4 zeros of padding;
- the XMLs' 121 `Scene` elements are 110 scenes and 11 unused headers;
- the layer scripts are checked at their XML symbols' offsets;
- the one-point tables are checked at their addresses through the name map, since main named the
  `D_` symbols.

**The pack, record by record** (`decomp-check.bat`: a loose import from each decomp, the old
records renamed through the map):

| Records | Count |
|---|---|
| Old pack / new pack | 12,202 / 12,885 |
| The same bytes | 11,904 (982 of them under a new name) |
| The same once their strings are renamed | 181 (83 of them the layer scripts keyed by offset, paired by their place in the scene records) |
| Different | 114 |
| Missing | 3 |
| New | 686 |

Every difference comes from main's XMLs or C correcting `2f4c25d`'s:
- **110 `tex/` records:** main's XMLs give these textures another size or format (the pause
  screen's maps, `gFieldDoor1Tex`, `gEffFleckTex`). The runtime draws from the meshes, which
  carry their own textures; the game never reads `tex/` records (only a pack test calls `GamePack::texture`).
- **`skel/object_door_killer/...`:** main corrects its limb type. Unused.
- **`table/cutscenes`:** the directory lists the 5 new scripts, and the layer scripts by name.
- **`table/item_drops`:** the 4 zeros of padding gone.
- **`meta/manifest`:** its counts.
- **The 3 missing:**
  - two meshes main's XMLs now start at 0, `object_lightbox_DL_000008` and
    `object_oA3_DL_00000008` (new as `..._DL_000000` and `..._DL_00000000`); both unused;
  - `gLinkChildUnusedBootTex`, a texture main no longer names.
- **The 686 new:**
  - 669 textures main's XMLs name;
  - 5 unused cutscene scripts (`gSpiritTempleUnused1Cs`, `gHyruleFieldZeldaEscapeUnusedCs`...);
  - 5 meshes (`sTransCircleDL`, `sTransWipeDL`, `gArmosUnusedDL`, the two above);
  - 4 collision headers and 3 animations.

The scenes' totals are unchanged: 110 scenes, 247 distinct headers, 655 rooms, 4,022 entries,
306,625 triangles (168,566 in the main headers), 0 unknown opcodes, the 4 unresolved references.

**The goldens.**
- Before re-recording, `golden.py check` gave 55 of 85 identical and 30 traces different.
- `python scripts/name_map.py compare-trace OLD NEW --packs out/loose-2f4c25d out/loose-new`
  shows each of the 30 is the baseline's bytes (`target/game13` at 0ed116d, on data12) with its
  names renamed.
- The renamed strings are in four fields:
  - `decomp_action`, Player's action (`func_80852E14` → `Player_Action_CsAction`);
  - `cutscene.script` (`D_808BCE20` → `gDekuTreeMeetingCs`, `0xA6D0` →
    `gKokiriForestIntroNaviFlyingCs`);
  - `place` (`ENTR_LINK_HOME_0` → `ENTR_LINKS_HOUSE_0`);
  - the audio log's `audRand` label (`sAudioRandom`).
- Every render is the same bytes. `viewer_link_sword`'s `--group SWORD` is now
  `SWORD_AND_SHIELD`, because main gave `PLAYER_MODELGROUP_SWORD` to group 15.
- Re-recorded and logged in [golden/README.md](../golden/README.md); the check then gives 85 of 85.

### Decisions

- **[ADR 0031](adr/0031-decomp-main.md):** the pin on `52a510f`, superseding ADR 0004. The
  name map is generated and kept in the repo, the citations follow it, and the importer reads
  main's layout. Pack format 16, the layer scripts keyed by the XMLs' names.
- **The cutscene layers' scripts keep the offset key only as a fallback.** Main's XMLs name all
  83, so `keys::cutscene_at` is now unused by the pack. It stays for a script no XML names.
- **`gTreasureChestSkel` stays a normal skeleton** (`objects::DRAWN_AS_NORMAL`). Main's XML
  declares it a flex skeleton, but `En_Box` draws it with `SkelAnime_Init` and `SkelAnime_Draw`;
  the record is the same bytes as before.
- **The old checkout stays.** The baseline build (a worktree at 0ed116d, `target/game13`) and the
  loose packs (`out/loose-2f4c25d`, `out/loose-new`) were for the proof, and are outside the
  repo or git-ignored.

### Known gaps

- **The extractor's sequence titles are empty** (`ootx extract`'s `sequences.json`, and the MIDI
  files' track names). `2f4c25d`'s `sequence.h` had a title per sequence in comments, and main's
  table has none.
- **Some `unk_` citations stay `unk_`.** They are fields main hasn't named
  (`PlayerAgeProperties`' `unk_24` to `unk_34`, `unk_94`), or a C struct the citation doesn't
  identify. [The review](name-map/README.md) lists what was checked.
- **The 110 differing `tex/` records** are main's definitions now. If a later port reads
  textures by name, it gets main's.
- **`csrc::Macros` evaluates what the importer needs**, not all of C: macros, enums and integer
  expressions. A table that needs more fails the import with the expression it couldn't
  evaluate.

### How to check

```bat
scripts\run\build.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\decomp-check.bat
```

- `decomp-check.bat` needs the baseline loose pack, `out\loose-2f4c25d`, which is already on this
  machine. Its header says how to make it again. It lists the records above and exits 1, since
  the explained differences are always there.
- `python scripts\name_map.py find func_80852E14` looks up a name, old or new;
  `python scripts\name_map.py find Player_Action_Talk` works the other way.
- `scripts\run\name-map.bat "D:\OOT Modding\OTT decomp\z64oot" "D:\OOT Modding\oot-main"
  "D:\OOT Modding\Debug Roms\baserom.z64"` regenerates the map (two Docker images and two full builds
  the first time). `git diff docs/name-map` should then be empty.

## Milestone 2: damage and health

**Answer:** done. Link takes every kind of hit the C has, flashes red while he's invincible,
and can die: the game over runs to "Continue?" and back to the entrance, or a bottled fairy
revives him. Enemies take damage from their damage tables. The exit holds:
- the training dummy hits Link, with each hit kind;
- a Deku Baba bites him (`En_Dekubaba`, ported whole but its effects), and he cuts it down for
  a Deku Stick.

The pack is format 17, in `out/data14` (Link's ice block). Decisions are in
[ADR 0032](adr/0032-damage-death-and-the-game-over-stand-in.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 45 to 49):
- `game-deku-baba.bat`: the game inside the Deku Tree, by a Deku Baba (`fairy` adds a bottled
  fairy);
- `game-dummy.bat KIND`: the training dummy, hurting Link with `none`, `fire`, `ice`,
  `electric` or `knockback`;
- `sandbox-deku-baba.bat`: the exit run headless, its trace and screenshots;
- `test-damage.bat`: the milestone's tests.

### What was built

**Player taking damage** (`oot_actors::player`). The body hit, the stagger, the knockdown and
the invincibility timer were already ported for the boulder (ADR 0020). New:
- **`func_80837C0C`'s other kinds:**
  - kind 3, frozen (`Player_Action_8084FB10`): the ice block, broken by mashing A or after its
    timer, with `NA_SE_PL_FREEZE_S` and `NA_SE_PL_ICE_BROKEN`;
  - kind 4, electrified (`Player_Action_8084FBF4`): 20 frames, the body's shock
    (`bodyShockTimer`, the sparks' sounds);
  - the hit while swimming (`Player_Action_8084E30C`, `func_80832594`'s knockback in the water).
- **Burning** (`func_8083819C`, `func_8083821C`, `func_80838280`, the body's flames): a fire hit
  sets Link burning, and dying without the Goron Tunic in a hot room or on a hot floor sets him
  burning again. A Deku Shield on his arm
  burns away (`Inventory_DeleteEquipment`), its pieces left as an `Item_Shield` placeholder.
- **The red flash** (`Player_Draw`): `damageFlickerAnimCounter` and `Gfx_SetFog2`, while the
  invincibility timer runs. The engine takes a per-draw fog for it (ADR 0032).

**Death and game over.**
- **Player:** at no health `func_80836448` starts the game over (`GAMEOVER_DEATH_START`), or the
  revival (`GAMEOVER_REVIVE_START`) when a bottle holds a fairy (`Inventory_ConsumeFairy`). The
  death animations on land and in the water (`Player_Action_80843CEC`,
  `Player_Action_8084E368`), the voice (`NA_SE_VO_LI_DOWN`), the letterbox, and `func_80843AE8`:
  the fall's end, or the revival's fairy and Link getting up.
- **`z_game_over.c`** (`oot_game::game_over`): `GameOver_Update` whole, its resets (the spoiling
  items, the temporary B button, the HUD), the game over lights (`Environment_InitGameOverLights`,
  `_FadeIn`, `_FadeOut`), and the revival's states 20 to 24.
- **The game over menu's stand-in** (`oot_game::kaleido`): the pause menu's game over states 8
  to 17 run with the C's timers and inputs, undrawn; "Continue? Yes" respawns at the entrance
  with three hearts. ADR 0032.
- **`Play_Update`'s order:** the game over and the pause menu stop the actors, the collision
  check and the cutscenes as the C's `IS_PAUSED` does; `GameOver_Update` runs in
  `Message_Update`'s place.
- **`Actor_UpdateAll`'s freezes:** the category masks by Player's state, the colour filter's
  timer, the finishing blow's freeze (`Enemy_StartFinishingBlow`).

**Enemy damage.**
- `DamageTable` and its lookup, the `DMG_*` flags, `DMG_ENTRY`.
- `CollisionCheck_ApplyDamage` with its four shapes (`CollisionCheck_ApplyDamageJntSph`, `Cyl`,
  `Tris`, `Quad`), and `CollisionCheck_SetATvsAC`'s special effect and backlash under main's
  names (`hitSpecialEffect`, `hitBacklash`).
- `Actor_ApplyDamage`, `Actor_SetColorFilter` and the filter's draw (`func_80026400`), and
  `Actor_SetDropFlagJntSph` with the drop table's override for an arrow's or magic's kill.

**`En_Dekubaba`** (`oot_actors::en_dekubaba`, pulled forward from milestone 3): every action,
both sizes, the damage tables, the drops (Deku Nuts, the Deku Stick), its floor shadow and its
skeleton. Its seven spheres follow the head and the stem through `sys_matrix.c`'s functions,
which `oot_game::sys_matrix` now has (`Matrix_Translate`, `_Scale`, `_RotateZYX`,
`_MultVec3f`...).

**The training dummy** (`dummy_target`): takes Link's hits through the Deku Baba's table, and
with `--target-hurts KIND` hurts him.

**Save presets:** `deku-tree-inside` (the Deku Tree's intro seen, `EVENTCHKINF_A8`) and
`deku-tree-inside-fairy` (a fairy in the first bottle).

### Results

**Tests.** `cargo test --release --workspace` (`target/game15`, `OOT_DATA_DIR=out/data14`): 345
passed, 0 failed, 1 ignored. 26 are new, with their expectations from the C:
- **`oot_game --test damage`** (13):
  - `Health_ChangeBy`, the double defence's shift, `Actor_ApplyDamage`;
  - `DMG_ENTRY`, the table's lookup (by the highest flag set);
  - a JntSph hit sphere by sphere (the C stops after the first unless `OC2_FIRST_ONLY`);
  - a hard collider, the special effect reaching its actor;
  - `Gfx_SetFog`'s multiplier and offset, the colour filter's fog over its duration;
  - `Inventory_ConsumeFairy`, `Inventory_DeleteEquipment`, the drop flag.
- **`oot_actors --test damage`** (13):
  - each hit kind's response and timers: the stagger, the knockdown, frozen, electrified (19
    frames after the hit's), burning and the shield burnt, the swimming hit;
  - the flash's fog following the invincibility timer;
  - death frame by frame: the game over states' run lengths (`DEATH_START`, `WAIT_GROUND`,
    `DELAY_MENU` 18 frames off the pause, the menu's states 8 to 14 with "GAME OVER" 30 frames,
    the delay 40, the window 10), then No, Continue, the fade's 26 frames, and the respawn;
  - the fairy's revival (states 22, 23, 24: 50, 64 and 50 frames);
  - the Deku Baba's tables for the Kokiri Sword, child and adult;
  - the exit's two runs, and Link's sword on the dummy.

**The exit run** (`Route::DekuBaba`, `--script deku-baba`): Link starts on the Deku Tree's top
floor, facing a Deku Baba 85 away. It bites him at frame 31 (`bitten`, half a heart), he hits
it while it's stuck after a missed bite (`baba_weakened`, 143), then cuts its stem
(`baba_cut`, 182). Its screenshots show the Baba drawn, the bite and the stick.

**The goldens.** Against milestone 1's build (`target/game14`, c806460, on data13):
- **`course_pit/sheet.png`** differs on its last six tiles (frames 48 to 58): the landing's
  fall damage now flashes Link red. Its trace is the same bytes.
- Every other trace and render is the same bytes. The freezes don't reach them, since no
  golden's actors run while Link talks to an actor that stops them or dies.
- **New case `deku_baba`:** the exit run's trace, 182 frames, the same bytes over two runs.

Re-recorded and logged in [golden/README.md](../golden/README.md): 86 hashes, 62 cases.

### Decisions

- **[ADR 0032](adr/0032-damage-death-and-the-game-over-stand-in.md):**
  - the game over menu's states run undrawn, "Continue? Yes" as the C;
  - `En_Dekubaba` ported whole now;
  - a per-draw fog in the engine for the flash and the colour filter;
  - `Actor_UpdateAll`'s freezes;
  - Player's new `Rand` calls on the game's generator through `PlayIo`;
  - the dummy's colliders;
  - pack format 17.
- **The renderer's fallback fog takes the camera's clip planes.** Where a scene has no fog (the
  test course), the fallback's range was 1 to 2, so the red flash's factor saturated. The
  scenes' own fog is unchanged.
- **The exit run starts on the Baba's floor,** not at the Deku Tree's entrance: the scripted
  run is the bite and the kill. The intro cutscene is skipped by the preset
  (`EVENTCHKINF_A8`), since it gets in the way of a run that starts inside.
- **The comment on the damage table's lookup was wrong.** It said several flags read the
  lowest; the C's loop shifts the flags right until they're 0, so it reads the highest. Only no
  flag at all is the bug (index 32, past the table).

### Known gaps

- **No effects:** the Deku Baba's dirt, dust and flames, Link's sparks and flames as particles,
  the ice's pieces. Their `Rand` calls aren't made (milestone 3, with `EffectSs`).
- **The game over menu isn't drawn**, and its "Save? Yes" doesn't write the save to SRAM.
  "Continue? No" respawns, where the C goes to the title screen. Milestone 5's pause menu.
- **No rumble** (`Rumble_Request`).
- **`Item_Shield`** (the burnt Deku Shield's pieces) is a placeholder, and the shield's bounce
  off an attack (`hitBacklash`'s recoil on Player) isn't ported.
- **The idle fidget keeps Player's own generator** (ADR 0032).
- **`ActorShadow_DrawCircle`** isn't ported for any actor; the Deku Baba draws only its own
  floor shadow.
- **One swing can hit the dummy on several frames:** it never changes state on a hit, where
  an enemy's reaction keeps the same swing from hitting it again.
- **The Deku Stick drops just below Link's reach** in the exit run's spot; the run doesn't pick
  it up.
- **No guarding with the shield (R).** Player's guard isn't ported (BACKLOG #4): the shield's
  collider (`shieldQuad`) is never registered, and Link can't block. It's milestone 3's, since
  the Deku Scrubs' nuts are beaten by deflecting them.

### Fixes after playing by hand

- **The death and revival cameras ignored Link** (BACKLOG #16). Dying with a bottled fairy, the
  camera went to the floor near the room's origin and stayed there after Link got up. Player
  starts the death's one-point cutscene (9806, `Camera_KeepOn4` on `CAM_SET_TURN_AROUND`) and
  the revival's (9908, `Camera_Unique9`) from his own update, where he's out of the actor arena
  (ADR 0007). So `OnePointCutscene_SetInfo` found no Player:
  - 9806's camera turned about (0, 0, 0);
  - 9908's setup returned early, so its camera never ran its keyframes, never timed out, and
    the main camera never got control back.

  Now the play state holds Player's camera view while his requests run
  (`PlayState::player_out_of_arena`), as the C has him in place. The fairy test checks both
  cameras and the return to the main camera. The crawlspace's 9601 and 9602 don't read Player,
  and every golden is unchanged.

### How to check

```bat
scripts\run\build.bat
scripts\run\test-damage.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-deku-baba.bat
```

By hand:
- `game-deku-baba.bat`: stand still and the Baba bites; Q (Z) locks on, E (B) slashes. Let it
  bite you down to nothing: the game over runs; at "Save?" in the console, D then Space for No;
  at "Continue?" Space, and Link starts again at the entrance.
- `game-deku-baba.bat fairy`: dying, the fairy revives Link.
- `game-dummy.bat ice` (or `fire`, `electric`, `knockback`, `none`): walk into the dummy.
