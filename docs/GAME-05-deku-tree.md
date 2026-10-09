# GAME-05: the Deku Tree

**Goal:** Phase 6 of the [roadmap](ROADMAP.md): the first dungeon, the Deku Tree, as the debug
ROM has it (Master Quest), up to Gohma's defeat. Decided in
[ADR 0028](adr/0028-phase-6-master-quest-and-the-decomp-upgrade.md): gc-eu-mq-dbg stays the only
ROM, and the decomp is upgraded first, before any dungeon work.

**Status:** done (2026-10-09). Every milestone below is built; the whole Deku Tree runs to
Gohma's defeat and the blue warp out in one test (milestone 6c).

| # | Milestone | Status |
|---|---|---|
| 1 | The decomp upgrade: an address-based name map from `2f4c25d`'s names to the new commit's, the citations migrated by it, the importer on the new layout, the pack's record names renamed (a format bump); every test passes and the goldens are the same bytes | done |
| 2 | Damage and health: Player taking damage (kinds 3 and 4, the hit while swimming, burning, the red flash), death and game over, the enemies' damage tables (`CollisionCheck_ApplyDamage`, `DamageTable`) | done |
| 3a | Combat basics: Player's guard with the shield (blocking, deflecting); `Camera_Battle1`; the effects (`EffectSs`, `z_effect.c`), `En_Dekubaba`'s included; `En_Firefly` (Keese, 7 placed) and `En_Karebaba` (withered Deku Baba, 5); drops on death | done |
| 3b | The rest of the MQ Deku Tree's enemies: `En_St` (2 placed), `En_Sw` (Skullwalltula and Gold Skulltula, 7), `En_Hintnuts`, `En_Dekunuts` and `En_Shopnuts` (3, 2, 1), `En_Goma` (eggs and larvae, 28, pulled forward from milestone 6) | done |
| 4 | Dungeon mechanics: `Door_Shutter` and small keys; switches, torches, webs; the map and compass; the `Bg_Ydan_*` actors; the crates (`Obj_Kibako2`); room-to-room travel. Split in three: 4a doors, switches, torches and webs; 4b the Deku Stick (pulled forward from 5) and the props; 4c pushing and Master Quest's extras | done |
| 5 | Items in use: Deku nuts (the sticks pulled forward to 4b), the Fairy Slingshot; the C buttons in full; a minimal pause menu for equipping; saving (`z_sram.c`). Split in three: 5a the slingshot and nuts; 5b the pause menu; 5c saving | done |
| 6 | Gohma: `Boss_Goma` (her larvae pulled forward to 3b); the boss room's camera and cutscenes; the heart container and the blue warp. **Exit:** a scripted run through the Deku Tree to Gohma's defeat. Split in three: 6a Queen Gohma, 6b the blue warp, 6c the run through the Deku Tree | done |

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

## Milestone 3a: combat basics

**Answer:** done. Link guards with the shield (R), blocking blows and bouncing a Deku Scrub's
nut back. Locked on to an enemy, the camera is `Camera_Battle1`. The game's effects are in:
the hit marks, dust, fragments, flames, sparks and the shield's streaks. The Keese
(`En_Firefly`) and the withered Deku Baba (`En_Karebaba`) are ported whole, and kills drop
items. The exit holds: in the Deku Tree's first room, Link kills a withered Deku Baba and takes
its Deku Stick, blocks a Keese's dive and kills the Keese, with the battle camera on; the kills
show their effects and the drops are picked up.

The pack is format 18, in `out/data15`. Decisions are in [ADR 0033](adr/0033-effects.md) (the
effects) and [ADR 0034](adr/0034-guard-battle-camera-and-the-first-enemies.md) (the rest).

**Scripts** (`scripts\run`, also in `menu.bat`, 50 to 52):
- `game-combat.bat`: the game on the Deku Tree's ground floor, by the withered Deku Baba and
  under the Keese;
- `test-combat.bat`: the milestone's tests;
- `sandbox-combat.bat`: the exit run headless, its trace and screenshots.

### What was built

**The guard** (`oot_actors::player`):
- **R guards:** `Player_ActionHandler_11` and `Player_Action_80843188`. The stick tilts the
  shield. B stabs from behind it (`func_808428D8`). Climbing, talking or picking up interrupts
  it. Letting go lowers it.
- **R while locked on** raises the shield on the upper body (`func_80834758`, `func_80834B5C`,
  `func_80834C74`).
- **The shield in hand** (`Player_SetModelsForHoldingShield`, `func_8008EC70`): the held
  shield's model in the pack (ADR 0034). Its collider, `shieldQuad`, is registered from the
  hand (`Player_UpdateShieldCollider`), wood for the Deku Shield.
- **A blow on the shield** (`func_808382DC`'s `AC_BOUNCED` path, `Player_Action_808435C4`): no
  damage, the recoil (the body's in the guard, the upper body's otherwise) and the push back at
  -18. A fire blow burns the Deku Shield.
- **The swing's contact** (`func_80842DF4`): a swing bounced off something hard rebounds
  (`func_80842D20`, `Player_Action_808505DC`). The blade meeting a wall strikes sparks (wood's,
  or metal's with the soft or hard wall's sound) and pushes Link back at 14. A hit on an actor
  freezes play a frame.
- **`Item_Shield`** whole: a Deku Shield lying about, and the burnt one, which takes the
  shield's place (`shieldMf`), hops, burns with eight flames and shrinks away.
- **`En_Nutsball`** whole, pulled forward from 3b: the Deku Scrubs' nut. It breaks on anything,
  and bounces back off the Deku Shield (or the adult's Hylian Shield) as Link's attack.
- `Player_SetupAction` stops the idle fidget's voices (`func_808326F0`) as the C does.

**`Camera_Battle1`** (`oot_game::camera`), whole: the swing to keep the enemy off to the side,
the pitch by the distance, the roll, the fov, the spin attack's charge, the target lost. With it,
`func_80043F94`, the bg check of rooms with the skybox disabled. BATTLE no longer falls back to
`Camera_Normal1`.

**The effects** (`oot_game::effect`, ADR 0033):
- `EffectSs` whole: the table, the spawn, the update, the draw, the
  `z_effect_soft_sprite_old_init.c` helpers;
- eight overlays: `Effect_Ss_Dust`, `_Hahen`, `_HitMark`, `_En_Fire`, `_En_Ice`, `_Dead_Db`,
  `_Fire_Tail` and `_Fhg_Flash` (the shock);
- `z_effect.c`'s sparks and shield particles.

Their callers now call them:
- the collision check's hit marks, blood and shield streaks;
- `En_Dekubaba`'s dirt, dust and flames;
- the chest's, the boulder's, the sign's and the Deku Tree's mouth's dust and fragments;
- Link's burning flames, shock sparks and the dust of his rolls.

**`En_Karebaba`**, whole: idle in its leaves, springing up, upright and spinning, retracting, a
slash killing it, the Deku Stick it leaves (offered for 200 frames), regrowing.

**`En_Firefly`**, whole: perched, the dive from the perch, flying about, the attack, the bite,
hovering after it, flying home. Hit, it is stunned, set alight, frozen or killed by its damage
table. Dying, it falls, shrinks and drops from table 14. Its fire and frost trail from the draw.

**`Actor_UpdateAll`** follows the C's list walk when an actor changes its category in its own
update (ADR 0034).

**The engine and the importer:**
- a draw's per-vertex colours (the sparks);
- bakes of overlay display lists (segment 0 as RAM holds the overlay);
- the held-shield Link variants;
- `MtxF` from glam's matrices.

### Results

**Tests.** `cargo test --release --workspace` (`target/game16`, `OOT_DATA_DIR=out/data15`):
359 passed, 0 failed, 1 ignored. New, with their expectations from the C:
- **`oot_actors --test guard`** (5):
  - the guard coming up, held and lowered: the animations by `modelAnimType`, the sounds, the
    collider at the hand's matrix;
  - a blow blocked: no damage, the recoil, the speed, then the guard again;
  - a Deku nut bounced back at the shield's yaw as `DMG_DEKU_STICK`;
  - a nut hurting Link without the guard and breaking into 15 fragments;
  - a fire blow burning the Deku Shield away (`Item_Shield`'s flames and timers).
- **`oot_game` `camera::tests`** (3):
  - `Camera_Battle1`'s data and timers, the swing and pitch it settles to, the fov at a heart
    or less;
  - the spin attack's charge (40 frames down to -20, back off to 250);
  - Z_PARALLEL when the target goes.
- **`oot_actors --test effects`** (1): a hit mark's life, colours, texture and draw, frame by
  frame.
- **`oot_actors --test enemies`** (4):
  - the withered Deku Baba's states frame by frame (the rise, upright, the spin's geometry, the
    retract);
  - its death by a slash, the stick, the regrowth;
  - the Keese's dive from its perch;
  - a slash killing a Keese, its fall and shrink, and table 14's drop.
- **`oot_actors --test combat`** (1): the exit run.

**The exit run** (`Route::Combat`, `--script combat`, from a debug start on the ground floor of
room 0):
- the withered Deku Baba springs up and is slashed at frame 107 (`karebaba_killed`);
- Link picks up its stick (`stick_taken`, 412);
- he locks on to the Keese and blocks its dive with the shield (`blocked`, 689);
- he slashes it while it hovers;
- it dies, and he picks up its drop (a green rupee: table 14's heart, at full health) at frame
  1890 (`keese_killed`).

The run shows 1,322 frames on the battle camera, 5 hit marks, 51 fragments and 5 dust clouds.

**The goldens.** Against milestone 2's build (`target/game15`, b9ce90c, on data14):
- **`course_target`** (its sheet and trace) and **`course_target_locked`**: locked on to the
  dummy, the camera is `Camera_Battle1`. The camera differs from frame 1. Link's moves follow
  the camera's input yaw from frame 16.
- **`deku_baba`**: the battle camera from frame 32 (the lock-on). The steering follows it, and
  the steps come 3 frames later (`baba_weakened` 145, `baba_cut` 184).
- **`playthrough`, `mido_shop`, `new_save_deku_tree`, `new_file_deku_tree`** and
  **`mido_shop_audio`**: each is the same up to the first slash that meets a wall (frame 865 by
  the bushes, 2838 at the plateau's switch). There the slash now recoils (`func_80842DF4`:
  `NA_SE_IT_REFLECTION_WOOD`, the wood's sparks, the speed -14), and the rest follows.
  - The playthrough's four bushes then gave no drop, so their order changed (the third and
    fourth swapped). Its bush step is now at frame 1137 (977 before).
  - The audio log's first difference is that sound. The voices `Player_SetupAction` now stops
    change nothing before it.
- **New case `combat`:** the exit run's trace, 1890 frames, the same bytes over two runs.
- Every other trace and render is the same bytes.

Re-recorded and logged in [golden/README.md](../golden/README.md): 87 hashes, 63 cases.

### Decisions

- **[ADR 0033](adr/0033-effects.md):**
  - the effects are play state, updated and drawn where the C does them;
  - the draws run once per game frame, for their `Rand` calls;
  - bakes with dynamic colours, per-vertex colours and overlay display lists;
  - the eight overlays the callers need.
- **[ADR 0034](adr/0034-guard-battle-camera-and-the-first-enemies.md):**
  - the held shield as a loadout bit, baked both ways;
  - `En_Nutsball` and `Item_Shield` whole;
  - `func_80043F94` for `Camera_Battle1`;
  - `Actor_UpdateAll`'s list walk;
  - pack format 18;
  - short scripted runs, the feel checked by hand.
- **The exit run is on the ground floor.** On the top floor, a withered Deku Baba's head flies
  back from where it faced and falls down the middle.
- **The exit kills the withered Deku Baba before the Keese.** While it's up, Z locks on to it
  rather than the Keese, and nowhere in the Keese's reach has it out of Z's view.

### Known gaps

- **Effects not ported:** the sword's trail (`EffectBlure`); `Effect_Ss_Fhg_Flash`'s light
  ball; every other soft sprite type, among them the bushes' and rocks' pieces
  (`EffectSsKakera`) and water splashes (`EffectSsSibuki`).
- **`func_80043F94` only for `Camera_Battle1`.** `Camera_KeepOn1` and `Camera_Parallel1` use
  `Camera_BGCheckInfo` in its place in rooms with the skybox disabled.
- **No lit torches:** `Obj_Syokudai` is a placeholder, so a Keese never catches fire from one.
- **`ActorShadow_DrawCircle`** is still not ported for any actor.
- **The guard's other shields:** the adult's guard and the Mirror Shield's are ported as the C
  has them, but untested (no adult route has a shield yet).
- **No rumble.**

### How to check

```bat
scripts\run\build.bat
scripts\run\test-combat.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-combat.bat
```

By hand, `game-combat.bat` (WASD the stick, Q is Z, E is B, R is R, Space is A):
- **The guard:** hold R with nothing locked on. Link raises the Deku Shield in his right hand
  (his sword stays on his back). Tilt the stick: the shield turns. Let go: it comes down.
- **The withered Deku Baba:** walk towards it (ahead at the start, a little to the side, by the wall); it springs up and
  spins its head. Q to lock on: the camera swings behind Link and keeps the Baba off to the
  side. E while it's upright: a white hit flash, dirt and dust; its head flies off and lands as
  a Deku Stick. Space by the stick picks it up. It grows back after a while.
- **The Keese:** it dives from the wall above. Lock on and hold R: the dive hits the shield
  (sparks, no damage, Link pushed back), and the Keese hovers. Let go of R and press E. It falls,
  shrinks away, and may leave an item. Without the shield, its bite costs half a heart.
- **A slash at a wall** recoils with sparks and a knock.
- **What to report:**
  - whether blocking feels right: when R takes effect, how far Link is pushed;
  - whether the battle camera feels right as you move round an enemy;
  - anything that looks off in the effects.

## Milestone 3b: the rest of the Deku Tree's enemies

**Answer:** done. Every enemy the Master Quest Deku Tree places is ported whole:
- the Deku Scrubs: `En_Dekunuts` (the Mad Scrub), `En_Hintnuts` (the hint scrubs and their order
  puzzle) and `En_Shopnuts` (the Business Scrub), with what a caught Business Scrub becomes
  (`En_Dns`, the salesman);
- the Skulltula (`En_St`);
- the Skullwalltula and the Gold Skulltula (`En_Sw`), with the token a Gold Skulltula leaves
  (`En_Si`), the save's token count and Gold Skulltula flags;
- Gohma's eggs and larvae (`En_Goma`, pulled forward from milestone 6), its boss side waiting
  for `Boss_Goma`.

Four effect overlays they call are ported whole too (`Effect_Ss_Fcircle`, `_Blast`, `_K_Fire`,
`_Sibuki`). The exit holds; its run is the golden `scrub`.

The pack is format 19, in `out/data16`. Decisions are in
[ADR 0037](adr/0037-the-deku-trees-other-enemies.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 53 to 55):
- `test-enemies.bat`: the milestone's tests;
- `game-enemies.bat NAME`: the game from a debug start in an enemy's room (`scrub`, `hint`,
  `shop`, `skulltula`, `walltula`, `gold`, `larva`);
- `sandbox-scrub.bat`: the exit run headless, its trace and screenshots.

### The exit

Proposed (change it if you meant something else): from debug starts in their rooms,
- Link bounces a Deku Scrub's nut back off the Deku Shield, which knocks it out of its flower,
  catches it and kills it (the Mad Scrub in room 4: the scripted run and golden);
- he kills a Skullwalltula, a Gold Skulltula (and collects its token) and a Skulltula;
- a Gohma egg hatches and he kills its larva.

"Catches it" is read as the Mad Scrub's: knocked out, it runs, and Link runs it down and slashes
it. The hint scrubs' catch (a talk) and the Business Scrub's (the salesman) are covered by the
tests and the hand tests. The other kills are C-derived tests (each from a debug start in its
room) and hand tests, not scripted runs (feedback: no long runs against `Rand`-driven enemies).

### What was built

**The Deku Scrubs** (`oot_actors::en_dekunuts`, `en_hintnuts`, `en_shopnuts`, `en_dns`):
- **The Mad Scrub:** waiting in its flower (its timer from `Rand`), up to spit nuts at Link
  (`En_Nutsball`, its nose swelling), down when he's near; its own nut bounced back knocks it out:
  it runs away three times, gasping between, then home; a Deku Nut's flash stuns it, fire sets it
  in a ring of fire, a sword kills it (the white puff, 15 fragments, a drop from table 3). Its
  flower is its child and becomes a prop when it dies.
- **The hint scrubs:** the same in the ground, but only their own nut knocks them out.
  `sPuzzleCounter` (an overlay static, on the play state) counts room 9's three in order: out of
  order, the third wrong one plays the error chime and the three sink and come back; in order,
  the third becomes friendly (`ACTORCAT_BG`) and runs, and caught (by touch, or Z) it talks
  (0x109C), leaves a recovery heart, runs off, and clears the room; the other two sink and are
  gone. `Actor_SetTextWithPrefix` is ported (`oot_game::npc`).
- **The Business Scrub** peeks and spits; hit by anything it spins up out of the ground and is
  replaced by the salesman (`En_Dns` with its params, room 3's 4: the Deku Shield for 50). He
  offers to talk; his choice checks the sale (`EnDns_CanBuy*`: not enough rupees, already owned,
  not usable yet), offers the item and is paid when Link has it; bought or not, he burrows down
  in dust (`func_80028990`), leaving three recovery hearts after a sale.

**The Skulltula** (`oot_actors::en_st`): on its thread bobbing; Link near and below, it drops,
lands with a shockwave (`Effect_Ss_Blast`), waits turning to face him and away, laughing, its
teeth flashing; back up when he leaves. Its six cylinders: the metal front sways it, the back
or body hurts it (2 health), a Deku Nut stuns it; dead, it bounces three times, rolls over,
burns away in seven flames and drops from table 14. Touching Link costs half a heart and
knocks him down (`play->damagePlayer`, ported as `player::play_damage_player`). Its spin's trail
(`EffectBlure`) isn't ported (ADR 0033, 0037).

**The Skullwalltula and the Gold Skulltula** (`oot_actors::en_sw`, `en_si`): a Skullwalltula finds its wall 60 behind it and turns on
it now and then (its turns and waits from `Rand`); with Link climbing within 130 and in its
sight it laughs, turns to him and dashes down the wall at him, purple-fogged, then brakes and
goes home. One hit kills it: it tumbles down, bounces twice with rings of dust, dissolves in nine
puffs and drops from table 3. A Gold Skulltula walks its wall or floor turning on the spot (one
of type 2 only at night; 3 and 4, spawned by others, jump out); two hits kill it: it spins,
dissolves, and leaves its token (`En_Si`), which grows and spins until Link touches it:
`Item_Give(ITEM_SKULL_TOKEN)` (the count and the quest bit), Link held for the text 0xB4 and the
small item fanfare, and its Gold Skulltula flag set (`SET_GS_FLAGS`) when the text closes. A
Gold Skulltula whose flag is set isn't spawned again. The save keeps `gsFlags`; the messages'
token count (`MESSAGE_TOKENS`) reads it. Ported with them: `Actor_SpawnFloorDustRing`,
`func_8002DDF4`, `SurfaceType_IsIgnoredByProjectiles`, and the Gold Skulltula's look as a bake
(the skeleton with its ten gold limbs).

**Gohma's eggs and larvae** (`oot_actors::en_goma`): an egg squishes and sheds fragments; Link
within 100 for 10 frames, it falls (a ceiling egg drops) and hatches into a larva, its shell 15
pieces of debris (`En_Goma` 10 to 24). The larva stands, chases Link, crouches and jumps at him;
the shield knocks it back or out of its jump, a Deku Nut stuns it, a sword's hit sprays bubbles
(`Effect_Ss_Sibuki`) and throws it back to flee; at no health it dies, a flame rises
(`Effect_Ss_K_Fire`), it shrinks away and drops from table 3. A hit breaks an egg before it
hatches. Its boss side is ported and waits for `Boss_Goma` (milestone 6): her eggs (params 0 to
2), her pieces (100 and up), the writes into her `childrenGohmaState` through a marked hook.
`En_Goma`'s profile carries `ACTOR_BOSS_GOMA`'s id, as the C's does, and every egg and larva
gets it.

**The effects** (`oot_game::effect`): `Effect_Ss_Fcircle`, `_Blast`, `_K_Fire` (on `_En_Fire`'s
bake) and `_Sibuki`, whole with their bakes; `CollisionCheck_WaterBurst` now spawns its bubbles;
`Effect_Delete`; the blast's draw reads the floor (`DrawCtx::col`).

**Elsewhere:**
- `CollisionCheck_GetSwordDamage`; Player's `unk_860` (always 0 until a Deku Stick can burn);
  `func_80028894` and `func_80028990`.
- Debug starts in a room: `Route::debug_start` (the room requested, a frame, the change
  finished, then Link placed), as the game's and the sandbox's `--room`.
- The draw-time `Rand` of a hurt larva runs in `draw_update`, once per game frame (ADR 0037).
- Per-limb colours as dynamic segments in skeleton bakes (the Skulltula's teeth, the larva's eyes
  and body); the Business Scrub's nose drawn apart while it spits.

### Results

**Tests.** `cargo test --release --workspace` (`target/game17`, `OOT_DATA_DIR=out/data16`):
447 passed, 0 failed, 1 ignored. New, with their expectations from the C:
- **`oot_actors --test scrubs`** (6):
  - the Mad Scrub's init (its shots, its table, its timer, its flower as its child), up (the
    collider's height frame by frame, `NA_SE_EN_NUTS_UP` on frame 8, `AC_ON` on 9), its stand,
    its spit (the nut 23 ahead and 12 up on frame 6), its next round and its burrow;
  - its own nut bounced back by Link's guard knocking it out (37 high, mass 50), its run (speed
    to 7.5 by 1), its gasp, a slash (knocked back at 10, red for the damage animation), its death
    (the puff, 15 fragments, its flower a prop);
  - a Deku Nut's stun (five loops, four faints, blue) and fire's ring (`EffectSsFCircle_Spawn`
    40 by 50), their hits injected;
  - the hint scrubs' puzzle: their texts (0x1000, 0x1000, 0x109C), a sword's hit (down, not
    counted), out of order (-1, -2, -3, the error chime, -4, all three back up), in order (1, 2,
    then the third friendly in `ACTORCAT_BG`), caught and talking, the heart, the room cleared,
    the other two gone, the flowers props;
  - the Business Scrub into the salesman (`En_Dns` 4: 0x10CB, 50, the Deku Shield), the choice,
    "already owned" (0x10A6), his burrow (`NA_SE_EN_AKINDONUTS_HIDE`, 0x2000 a frame, gone 400
    down, nothing paid), and one selling a piece of heart already bought gone at init;
  - the Deku Shield sold: 0x10A7, the offer, Link holding it up, paid 50 when the text is done,
    three hearts.
- **`oot_actors --test skulltula_st`** (9): its colliders, scale and ceiling; the bob by the frame
  counter; the drop, the shockwave and the landing, waiting and turning; back to the ceiling; a
  slash on its front (the sway, no damage) and on its back (the spin, red); the killing slash's
  three bounces, the roll and seven flames; a Deku Nut's 120-frame stun; touching Link.
- **`oot_actors --test skulltulas`** (6): the Skullwalltula finding its wall and turning; its dash
  at Link climbing its vines; a slash killing it (the fall, the bounces, nine puffs, table 3); a
  Gold Skulltula's two slashes and its token; the token taken (the count, the freeze, 0xB4, the
  fanfare, the flag) and the Gold Skulltula not spawned again; the spawned ones jumping out or
  waiting for the night.
- **`oot_actors --test gohma_larvae`** (9): the eggs' init (`sSpawnNum`, their `Rand` values);
  the squish and the fragments every 16 frames; hatching after 10 frames near (the 15 pieces of
  shell); the chase and the jump; two Kokiri Sword slashes (the bubbles, the hurt, the flight,
  the death, the flame, the shrink, table 3); an egg broken; a Deku Nut's stun; the shield's
  knockback; the ceiling egg's fall.
- **`oot_actors --test scrub_run`** (1): the exit run.
- Two older tests changed: `enemies`' Keese kill now allows a heart turned into a green rupee
  at full health (`func_8001F404`, as the combat test did), and `scenes` finds a profile's row by
  name (`En_Goma`'s id is `Boss_Goma`'s) and reads `En_Sw`'s params after its init's conversion.

**The exit run** (`Route::Scrub`, `--script scrub`): Link guards from frame 1; the Mad Scrub
spits and its nut comes back off the Deku Shield into it (`nut_bounced` 73); he locks on and runs
it down as it runs and gasps, and slashes it (`scrub_caught` 142); it dies (`scrub_killed` 170,
no drop this time). 170 frames.

**The goldens.** Against milestone 3a's build (`target/game16`, 552ce5d, on data15):
- **`playthrough`, `mido_shop`, `new_save_deku_tree`, `mido_shop_audio`**: Kokiri Forest's Gold
  Skulltula (out only at night) was a placeholder; it now calls `Rand`, so the playthrough's
  first bush drops (the bush step at 1085, not 1138), and the plateau switch's rupee lands on
  Link (the switch step 6 frames sooner, the rest 14 later).
- **`deku_baba`, `combat`**: room 0's Skullwalltula's init moves it into the enemy list during
  `Actor_UpdateAll`, and the loop goes on there as the C's does, so room 0's enemies update a
  frame sooner; its new actors call `Rand`. The Deku Baba bites at 30 (31). The combat run's
  Keese chase stalled and was fixed (a sidestep now keeps the chase going): `keese_killed` at 782
  (1890).
- **New case `scrub`:** the exit run, the same bytes over two runs.

Re-recorded and logged in [golden/README.md](../golden/README.md): 88 hashes, 64 cases.

### Decisions

- **[ADR 0037](adr/0037-the-deku-trees-other-enemies.md):** every enemy whole with what it
  becomes (`En_Si`, `En_Dns`); the four effects whole; `EffectBlure` still unported;
  `En_Goma`'s boss side behind a hook; draw-time `Rand` in `draw_update`; per-limb colours in
  bakes; debug starts change room first; the exit's run is the Mad Scrub's; pack format 19.
- **The ADRs' numbers:** 0035 and 0036 are the overworld editor's, so this milestone's is 0037.
- **Room 0's actors call `Rand` now:** 3a's combat route had to be re-tuned (below), and the
  goldens in the Deku Tree changed.
- **The Deku Tree's "big" Skulltulas are the normal size:** both placed have params 0 (params 1
  is the big one).

### Known gaps

- **`EffectBlure`** (the Skulltula's spin trail, and Link's sword's) isn't ported.
- **`Boss_Goma`** isn't: the larvae's hook logs its writes, and no boss piece has a bake yet.
- **Not reachable yet:** Deku Nuts, arrows, a burning Deku Stick, the Lens of Truth and the
  hammer's shock wave aren't Player's yet; the tests inject their hits where the enemies react
  to them (the stuns, the fire ring).
- **Room 2's Skulltula** hangs 468 above the floor below it; `EnSt_IsCloseToPlayer` allows 400,
  so it only drops for Link on the higher ground (as in the game).
- **The order of `Rand` in a call's arguments** (a larva's debris positions, its hurt colours)
  assumes IDO evaluates them left to right; it isn't checked against the disassembly.
- **`ActorShadow_DrawCircle`** is still not ported for any actor; no quake offset.
- **`En_Sw`'s**: the Gold Skulltula's shine is lit from the camera's view, not from the eye
  towards it (`func_8002EBCC`'s look-at); the room 0 Gold Skulltula's crate (`Obj_Kibako2`) is a
  placeholder, so it's out in the open; the Skullwalltulas sit above Link's reach until he has
  nuts or the slingshot; no hookshot (the token's pull).

### Fixes after playing by hand

- **`game-enemies.bat shop` dropped Link into a hole.** Its start, 250 to the Business Scrub's
  -x side, is over one of room 3's holes down to room 9; the floor check that picked it read
  another surface. It's now 220 off towards -x and -z, (-718, -820, 177), on the floor and in
  the 160 to 480 the scrub needs to come up. The tests' start was the same spot (they passed
  because they move Link next to the salesman soon after) and is moved too.
- **The hint scrubs' order isn't 2, 3, 1.** That's the original Deku Tree's: its third scrub
  tells it (text 0x109B, "The order is... 2 3 1", still in the ROM). Master Quest's room places
  the scrubs differently, and its third says 0x109C (Queen Gohma's weak spot) instead. The C
  counts the scrubs by their params, 1, 2, 3 (`EnHintnuts_HitByScrubProjectile2`), and the MQ
  scene puts params 1 at (-369, -1880, -904), 2 at (-947, -1880, -757) and 3, the one that
  talks, at (-660, -1880, -951): from the debug start (or from where Link drops in from room 3),
  facing them, the right one, then the left one, then the middle one.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-enemies.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-scrub.bat
```

### By hand

`game-enemies.bat NAME` (or menu 54). WASD the stick, Q is Z, E is B, R the shield, Space A.
1. **`scrub`** (room 4): the Mad Scrub pops up and spits at you. Hold R facing it: the nut bounces
   back into it, and it jumps out and runs. Q to lock on, run after it, E when close: a white
   puff and fragments, maybe a drop. Without R the nut costs half a heart. Walk within about 120
   and it hides.
2. **`hint`** (room 9): three scrubs spit at you. Knock each out with its own nut (R): first the
   one to your right, then the one to your left, then the one ahead. The first two freeze blue and
   faint; the third runs: catch it (walk into it, or Q) and it talks, leaves a heart and runs off;
   the other two sink away. In another order: an error chime, and all three sink and come back.
3. **`shop`** (room 3): the Business Scrub spits; knock it out with its nut: it spins up and
   stands, nervous. A by it: it offers the Deku Shield for 50; you have one, so it says so and
   burrows away in dust, spinning.
4. **`skulltula`** (room 5): it drops from the ceiling with a sound and a white ring on the
   floor, bobs, laughs, its teeth flash red, and it turns to face you and away. E on its front:
   sparks and a metal sound, it swings on its thread. E on its back: red, a cry, it spins and
   rises; a second back slash kills it (three bounces, it rolls over, seven small flames, maybe an
   item). Under it: half a heart and a knockdown. Walk away: it climbs back up.
5. **`walltula`** (room 0): look up at the vines: it turns now and then. Climb the vines (the
   stick into the wall) and keep moving: it laughs, turns purple and dashes down at you. Hold
   still on the vines: it goes back. (It can't be reached from the floor yet.)
6. **`gold`** (room 0's ledge): two slashes: red, then it spins and puffs away, a jingle, and a
   token rises spinning. Walk into it: Link freezes, the small item fanfare, "You destroyed a
   Gold Skulltula...". Leave the Deku Tree by its entrance and come back: it's gone.
7. **`larva`** (room 0's ledge): the ceiling egg drops onto the chest and hatches into a larva
   in a burst of shell; walk to the ledge egg: it falls and hatches. A larva runs at you, crouches
   (eyes red) and leaps. E: bubbles, it flashes and is thrown back, then flees; a second E kills it
   (it rolls over, a flame, it shrinks away, maybe an item). R as it leaps stops it.
- **What to report:** whether each enemy's timing feels right (the scrubs' spit, the Skulltula's
  drop and turns, the Skullwalltula's dash, the larva's leap); whether bouncing a nut back with
  R feels as it should; anything that looks off (the Gold Skulltula's shine and its token's size,
  the egg's squish, the bubbles' direction, the effects).

## Milestone 4: the dungeon's mechanics

**Status:** split agreed (2026-10-06). The user chose:
- three parts, 4a, 4b and 4c, as below;
- the Deku Stick pulled forward into 4b, used from C-Left only: a save preset and the Start
  stand-in put owned sticks there, and the C buttons in full stay in milestone 5;
- every actor whose use needs an item or song Link won't have (the time blocks, the rocks, the
  crates, the eye switches, room 2's ladder) ported whole, its trigger injected in the tests;
- the map and compass as data only, drawn by milestone 5's pause menu.

### The survey

The placements come from `ootx scene-info --scene ydan` (pack format 19, `out/data16`). Layers 0
to 3 all read the scene's one header (0x0), so this is every placement in the Master Quest Deku
Tree. Rooms are written as "front/back" for the transition actors (`sides[0]`, `sides[1]`).

**The transition actors:**
- **`Door_Shutter`, 9 doors** (`z_door_shutter.c`, 1,086 lines):
  - plain (type 0): rooms 1/0 and 8/7;
  - barred until the room is cleared (type 1, `SHUTTER_FRONT_CLEAR`): 11/9, 0/10 and 7/6;
  - barred until a switch flag is set (type 2, `SHUTTER_FRONT_SWITCH`): 2/1 on 0x0C (room 1's
    eye switch) and 5/4 on 0x19 (room 4's two timed torches);
  - type 7 (`SHUTTER_FRONT_SWITCH_BACK_CLEAR`): 6/5 on 0x09 (room 5's timed torches) and 4/3 on
    0x15 (room 3's eye switch).

  There is no key-locked door (type 11) and no boss door (type 5). MQ's Deku Tree places no
  small key, and the boss room is entered by a drop (exit 2).
- **`En_Holl`, 3, already ported:** the drops 0 to 3 and 3 to 9 (`ENHOLL_V_INVISIBLE`) and the
  plane between 7 and 3 (`ENHOLL_H_INVISIBLE`).

**The mechanisms:**

| Actor | Placed | What it does in MQ | C lines | Needs |
|---|---|---|---|---|
| `Obj_Switch` | 7 | Floor switches, pressed once: room 0's top floor (0x27: the web over room 10's door burns and room 0's golden torches light), room 3 (0x02: its golden torch; 0x14: its falling chest), room 10 (0x3D: the three rising platforms). Held down: room 5 (0x3E) and room 7 (0x38), their golden torches. Eye switches: room 1 (0x0C, the door to room 2) and room 3 (0x15, the door to room 4) | 848 | The eye takes only seeds and arrows (`0x0001F824`): the slingshot |
| `Obj_Syokudai` | 14 | Golden torches lit by a switch flag: room 0 ×3, room 3, room 5, room 7 ×4. Timed torches: room 4 ×2 (both lit: 0x19, the door to room 5), room 5 ×2 (0x09, the door to room 6), room 10 (0x13, its falling chest). Room 10 has a wooden torch that is always lit | 315 | Lighting one needs a burning Deku Stick, or fire arrows or Din's Fire |
| `Bg_Ydan_Sp` (webs) | 8 | Floor webs: room 0's bottom (a fall of over 750 onto it breaks it: the drop to room 3) and room 3's upper floor (burned: the drop to room 9). Wall webs: room 0's top, over room 10's door (burned by switch 0x27); room 0's middle floor, over room 1's door; room 2; room 3; room 7 ×2 | 467 | The wall webs but one, and room 3's floor web, burn only from a burning stick (`Player_IsBurningStickInRange`) or fire |
| `Bg_Ydan_Hasi` | 3 | Room 5's floating block and its water; room 10's three rising platforms (0x3D, up for 260 frames) | 202 | Nothing. The floating block is ported (the spikes'), but its init and the other two kinds aren't |
| `Bg_Ydan_Maruta` | 2 | Room 5's rolling spiked log; room 2's ladder, which falls when hit (0x21) | 218 | The ladder takes only a seed (`DMG_SLINGSHOT`) |
| `Obj_Kibako2` (crates) | 3 | Room 0's middle-floor crate, with the Gold Skulltula (`En_Sw` 0x8102) placed inside it on its own (the crate's params 0xFFFF spawn none); room 10 ×2 | 189, and `Effect_Ss_Kakera` 436 | It breaks only on an explosion (`func_80033684`) or the hammer (`0x40000040`) |
| `Obj_Lift` | 1 | Room 2: a platform that shakes when stood on, falls and breaks (0x20) | 240 | Quakes and `Effect_Ss_Kakera` |
| `Obj_Timeblock` | 10 | The Song of Time's blocks: room 2 ×4 and room 7 ×5 hidden (no collision); room 5 ×1 standing on the purple rupee's chest | 367 | The ocarina. The song's effect (`Demo_Effect`, 2,092) spawns only on the song |
| `Obj_Bombiwa` | 3 | Room 2's rocks | 158 | Bombs (or the hammer) |
| `Obj_Makeoshihiki` | 1 | Room 3: spawns a small push block (`Obj_Oshihiki`) on the upper floor; pushed off into the pit, it sets 0x10 and stays | 144, and `Obj_Oshihiki` 688 | Player's push and pull (`Player_Action_8084B78C`, `_8084B898`, `_8084B9E4`: not ported; `func_8083F72C` only logs) |
| `Bg_Haka` | 8 | Room 7's gravestones, pulled back 60 | 174 | Player's pull |
| `Elf_Msg`, `Elf_Msg2` | 8, 4 | Navi's hints: places where she calls, and tags to check with C-Up | 230, 194 | Nothing: Navi's forced talks and C-Up are ported |
| `En_Wonder_Item` | 4 | Room 7 | ported | |
| `En_Box` | 7 | The map (room 0's ledge, flag 3), the compass (room 2), the slingshot (room 10, when the room is cleared), two Deku Shields falling on 0x14 (room 3) and 0x13 (room 10), the purple rupee (room 5, under the time block), a recovery heart (room 5) | ported | |

**Elsewhere:**
- **Small keys.** `Item_Give` already counts them in the save. The HUD's key count
  (`Interface_Draw`) isn't drawn, and nothing in MQ's Deku Tree gives or takes one.
- **The map and compass.** `Item_Give` already sets them in `dungeonItems`. The pause data isn't
  ported:
  - `z_map_exp.c` (635 lines): `Map_Init`, `Map_InitData` and `Map_Update`, which record the
    rooms visited and the floor Link is on;
  - `z_map_data.c` (351): the dungeons' tables;
  - `z_map_mark.c` (167): the chests' and the boss's marks.

  The HUD's minimap (`Minimap_Draw`) is a cross-cutting debt.
- **Quakes** (`z_quake.c`, 509 lines) aren't ported (ADR 0029). `Door_Shutter`'s slam and
  `Obj_Lift`'s shake call them.
- **Player's sliding door** (`Player_ActionHandler_1`'s `PLAYER_DOORTYPE_SLIDING` branch, the walk
  through it in `Player_Action_80845CA4`) only logs today. So do the Deku Stick
  (`Player_InitDekuStickIA`, `Player_UpdateBurningDekuStick`, its breaking) and push and pull.

**What Link can't do yet, and what it closes off:**
- **The slingshot (milestone 5):** the eye switches in rooms 1 and 3 (the doors to rooms 2 and
  4) and room 2's ladder. So rooms 2 and 4 to 8 stay debug starts until milestone 5.
- **The Deku Stick** (also milestone 5 on the roadmap): every torch Link lights, the wall webs but
  the one switch 0x27 burns, and room 3's floor web (the way to rooms 9 and 11).
- **Push and pull:** room 3's block and room 7's gravestones.
- **The ocarina:** the time blocks.
- **Bombs:** the rocks, and the crates, so room 0's Gold Skulltula stays shut in its crate.

Without the stick, Link reaches rooms 0, 10 and 3. With it, he also reaches room 1 and rooms 9
and 11 (room 9's door to room 11 opens when the hint scrubs' puzzle clears the room).

### The split

Each part ports its actors whole, with C-derived tests per actor, and keeps one short scripted
run as its exit and golden. A trigger Link can't use yet (a seed, a song, an explosion) is
injected in the tests where the actor reacts to it.

**4a: doors, switches, torches and webs**
- `Door_Shutter` whole: every type and style, the Gohma block and Phantom Ganon's bars as their
  collisions, the Jabu Jabu and boss doors' draws baked. With it:
  - Player's sliding door: the A press, the walk through, the door camera, the room swap and
    `Room_FinishRoomChange`, the respawn point, and the wait when the door bars behind him
    (cutscene actions 2 and 7);
  - `Actor_DrawDoorLock` and the small keys' count on the HUD (tested on a key-locked door that a
    test spawns).
- `z_quake.c` whole: the cameras' shake, for the door's slam and `Obj_Lift`, and for the one-point
  cases ADR 0029 left out for want of quakes.
- `Obj_Switch` whole: floor (once, toggle, held), rusty, eye and crystal. The eye's seed hit is
  injected in the tests.
- `Obj_Syokudai` whole: golden, timed and wooden torches, their light and flame. A Keese set
  alight at one (`EnFirefly_ApproachLitTorch`, already ported) now works. Lighting from a stick
  reads Player's held item, so it waits for 4b; fire is injected in the tests.
- `Bg_Ydan_Sp` whole: the floor webs bounce under Link, break under a fall of over 750, and
  rewrite their collision's vertices each frame (the engine gets a DynaPoly whose vertices change;
  the C writes the object's shared header, so both floor webs share it). The wall webs burn on a
  switch flag or fire. The burning stick's checks (`Player_IsBurningStickInRange`) are ported and
  reached in 4b.
- `Elf_Msg` and `Elf_Msg2` whole.
- The map and compass in the pause data: `z_map_exp.c`'s data side, `z_map_data.c`'s tables in
  the pack (format 20), and `z_map_mark.c`'s marks; drawn in milestone 5.
- A debug start for every room, each checked by placing Link and running frames (he stands, and
  the room doesn't change).
- **Exit:** from a debug start on room 0's top floor, Link steps on the switch: the web over room
  10's door burns (the chime, the flames) and the three golden torches light, with the attention
  cameras. He opens room 10's sliding door and walks through, the room changes, and the door
  slams and bars behind him (room 10's enemies are alive), with his pause. That run is the golden.
  A C-derived test covers the drop through room 0's floor web into room 3.

**4b: the Deku Stick and the props**
- The Deku Stick, pulled forward from milestone 5: in hand (its bakes), swung as a weapon
  (`DMG_DEKU_STICK`, its length), broken (`unk_85C`, the piece that falls, its ammo), lit at a
  torch and burning (`Player_UpdateBurningDekuStick`: 210 frames, the flame at its tip, the stub
  burnt down), put out in water. It lights the timed torches and burns the webs. It's used from
  C-Left (`Player_ProcessItemButtons`' C buttons); a save preset and the Start stand-in put owned
  sticks there.
- `Bg_Ydan_Hasi` whole: the water, the floating block and the three rising platforms. The spikes'
  sandbox platform is rebuilt on its init.
- `Bg_Ydan_Maruta` whole: the rolling log. The ladder's fall on a seed is injected in the tests.
- `Obj_Kibako2` whole, and `Effect_Ss_Kakera` whole with its callers that are already ported
  (`En_Kusa`'s and `En_Ishi`'s pieces, a cross-cutting debt). Their `Rand` calls shift the Kokiri
  Forest runs: expect golden changes there.
- `Obj_Lift` whole.
- **Exit:** in room 0, Link lights a Deku Stick at a golden torch on the middle floor, burns the
  web over room 1's door, and goes through the door. That run is the golden. C-derived tests
  cover room 4's two timed torches opening its door, room 10's timed torch dropping its chest, and
  a stick breaking on a hit.

**4c: pushing and Master Quest's extras**
- Player's push and pull: `func_8083F72C` and the push and pull actions, `PLAYER_STATE2_4`, the
  block's pull on Link (`dyna.unk_150`), and `CAM_MODE_PUSH_PULL`.
- `Obj_Oshihiki` whole and `Obj_Makeoshihiki` (room 3's block); `Bg_Haka` whole (room 7's
  gravestones).
- `Obj_Timeblock` whole: shown or hidden with its collision, from its flags. The song's side
  waits for the ocarina, and its `Demo_Effect` stays a placeholder (it spawns only on the song).
- `Obj_Bombiwa` whole. The explosion is injected in the tests.
- Room-to-room travel made solid: a C-derived test walks the connections milestone 4 opens
  (0 to 10, 0 to 1, the drops 0 to 3 and 3 to 9, then 9 to 11 once the room is cleared) through
  the real doors and drops, from one start.
- **Exit:** in room 3, Link pushes the block off the upper floor into the pit (0x10, the chime)
  and climbs onto it. That run is the golden.

The new decisions go in ADRs from 0038. Every part's scripts, docs and goldens are as in 3b.

## Milestone 4a: doors, switches, torches and webs

**Answer:** done. The MQ Deku Tree's sliding doors open, bar and unbar; its switches, torches,
webs and Navi's hint spots work; the camera shakes; the map and compass are recorded. Every
actor is ported whole, including the paths a Deku Stick, a seed or an arrow will reach later;
the tests inject those. The exit holds; its run is the golden `shutter`.

The pack is format 20, in `out/data17`. Decisions are in
[ADR 0038](adr/0038-sliding-doors-and-room-travel.md) (the doors and room travel),
[ADR 0039](adr/0039-quakes.md) (quakes) and
[ADR 0040](adr/0040-switches-torches-webs-and-the-map-data.md) (the rest).

**Scripts** (`scripts\run`, also in `menu.bat`, 56 to 58):
- `test-mechanics.bat`: the milestone's tests;
- `game-dungeon.bat WHERE`: the game from a debug start: `switch` (room 0's top floor by the
  floor switch, the default), `lobby-top`, `lobby`, or `room1` to `room10`;
- `sandbox-shutter.bat`: the exit run headless, its trace and screenshots.

### What was built

**The sliding doors** (`oot_actors::door_shutter`, ADR 0038):
- `Door_Shutter` whole: every type (plain, barred until the room is cleared or a switch is set,
  locked by a small key or the boss key, unopenable from behind), every style (the Deku Tree's
  doors and the other dungeons', the Jabu Jabu door's eight sections, the boss door's texture per
  dungeon), the bars, the slam with its dust and quake, Gohma's slab and Phantom Ganon's bars with
  their collision.
- Player's sliding door: the A press, the walk 20 to the door and 120 past it, the door camera,
  the room behind loaded and the old one dropped, the respawn point, and the pause when the door
  bars behind him (cutscene actions 2 and 7).
- `Player_ProcessSceneCollision` now picks its bg check flags as the C does, so a door's walk
  skips the doorway's wall.
- A room's last enemy gone sets its temporary clear flag (`Actor_RemoveFromCategory`), which
  unbars its doors and shows its room-clear chests.
- `Actor_DrawDoorLock` (both doors draw it), and the HUD's small key icon and count, in the
  scenes `Interface_Draw` lists (not the Deku Tree).
- DynaPolyActor's interact flags (`DynaPolyActor_IsPlayerOnTop` and the like), set by Player and
  `Actor_UpdateBgCheckInfo`, cleared after the owner's update.

**Quakes** (`oot_game::quake`, ADR 0039): `z_quake.c` whole, run in `Camera_Update`, the shake added
to the view. The door's slam, Player's roll into a wall and damaging fall, `En_Goroiwa`'s drop and
the cutscenes' quake commands call it.

**Switches, torches, webs and Navi's tags** (ADR 0040):
- `Obj_Switch` (`obj_switch`) whole: floor (once, toggle, held), rusty, eye and crystal; the frozen
  kind's ice (`Obj_Ice_Poly`) is a placeholder. Room 0's top-floor switch burns the web over room
  10's door and lights the golden torches.
- `Obj_Syokudai` (`obj_syokudai`) whole: golden torches lit by a switch flag, timed ones lit by fire
  (several lit at once set their flag), the wooden one always lit, the flame and its point light,
  and a Keese catching fire at a lit torch (`En_Firefly`'s hook now reads the torch).
- `Bg_Ydan_Sp` (`bg_ydan_sp`) whole: the floor webs bounce under Link, break under a fall of over
  750 (the drop from room 0's top floor to room 3), and rewrite their collision's vertices each
  frame (an object header shared through `Dyna::replace_shared_header`); the wall webs burn on a
  switch flag or fire.
- `Elf_Msg` and `Elf_Msg2` (`elf_msg`, `elf_msg2`) whole: the spots where Navi speaks up, and the
  tags to check with C-Up.

**The map and compass** (`oot_game::map`, ADR 0040): `z_map_exp.c`'s data side (the rooms visited,
the floor Link is on, the palettes, `mapIndex`), with `z_map_data.c`'s tables and MQ's map marks
in the pack (`table/map`, checked against the ROM). Nothing is drawn: milestone 5's pause menu
will.

**Debug starts:** `playthrough::DEKU_TREE_ROOM_STARTS`, one per room but room 11 (its whole floor is
the drop to Gohma), plus one on room 0's top floor.

### Results

**Tests.** `cargo test --release --workspace` (`target/game18`, `OOT_DATA_DIR=out/data17`): 501
passed, 0 failed, 1 ignored. 54 are new, with their expectations from the C:
- **`oot_actors --test doors`** (7): the doors room 0 spawns (types, style, object, the init's
  `home.pos.z` quirk); through room 10's door frame by frame (the offer, the walk's targets, the
  door camera, 3 to 15 a frame up to 200, the slam at 30 a frame, 11 dust clouds, the respawn
  point, 32 frames held); room 10's clear unbarring it (the attention cameras, the bars in 6
  frames, as f32 steps them); room 1's door barred by its eye switch's flag and back; a key spent
  on a key-locked door (10 frames of unlocking); the draw from Link's side only, and the lock's
  chains; Gohma's slab falling and bouncing in her room.
- **`oot_actors --test switches`** (7): room 0's floor switch (pressed, the attention camera, down,
  staying down), room 5's held switch, room 3's eye switch hit with a seed (from the right side
  only), toggle floor and crystal switches, the targetable crystal and the frozen eye.
- **`oot_actors --test torches`** (6): room 0's golden torches lit by 0x27, room 4's timed pair
  setting 0x19, one alone burning out, room 10's wooden and timed torches, the Deku Stick's paths,
  a Keese catching fire.
- **`oot_actors --test webs`** (9): the floor web's swing and bounce, broken by a long fall with Link
  through into room 3, burnt by its switch flag, a burning stick or fire, a destroyed web staying
  gone.
- **`oot_actors --test elf_msg`** (4): Navi's forced text, the kills on flags, `Elf_Msg2` through C-Up.
- **`oot_actors --test map`** (5): entering the Deku Tree, a room change, the floor from Link's
  height, Kokiri Forest, the map and compass chests.
- **`oot_actors --test debug_starts`** (1): every room's start; **`--test shutter_run`** (1): the
  exit run.
- **`oot_game`**: 11 quake tests (each type frame by frame, the countdown, the table's limits, the
  camera's view shaken) and the key counter's.
- **`oot_import --test pack`**: the map tables against the ROM; **`eng_collision`**: the shared
  header.
- Five older tests changed where the new actors meet them, each the game's behaviour:
  - `switches`: flag 0x27 also burns the web (a second chime the same frame) and lights the
    golden torches (more attention cameras to wait out);
  - `scrubs`: solving the hint scrubs' puzzle clears room 9, whose door to room 11 now unbars with
    attention cameras that hold the actors a while;
  - `skulltulas`: the Skullwalltula's vines and the Gold Skulltula's ledge are in Navi's hint
    spots (`Elf_Msg` 0x1F02): the tests set their flag, as if heard;
  - `torches`: the tests clear their room, which now unbars its doors with their own attention
    cameras;
  - `gohma_larvae`: an egg's fragments come by `Rand` (half a chance every 16 frames); with room 0's
    torches drawing too, 128 frames happened to bring none, so the test watches 512.

**The exit run** (`Route::Shutter`, `--script shutter`, from a debug start on room 0's top floor,
by the switch): Link steps on the switch (`switch_pressed` 49); the web over room 10's door burns
and the golden torches light with their attention cameras (`web_burnt` 114). On the way to the
door the Keese perched above it dives and bites him (half a heart), and at the door Navi speaks
up (`Elf_Msg` 0x0103, text 0x103). He opens the door (`door_opened` 525), walks through, the door
slams (548) and bars behind him (room 10's enemies are alive), and he's let go (`door_barred`
581). 581 frames.

**The goldens.** Against milestone 3b's build (`target/game17`, cd5f68f, on data16):
- **`course_run-roll`, `course_run-roll_child` and `course_pit` sheets:** the camera shakes on the
  roll into a wall and the damaging fall (quakes); their traces are the same bytes.
- **`playthrough`, `new_save_deku_tree`, `new_file_deku_tree`:** the same until the end of the Deku
  Tree's intro, then the dungeon camera's eye sits lower: room 0's floor web has collision now.
  (Without `Bg_Ydan_Sp` registered they're the baseline's bytes.)
- **`scrub` and `combat`:** room 4's two and room 0's three torches call `Rand` at init. The scrub's
  run differs from frame 151 in the battle camera only; the combat run's steering from frame 432,
  `blocked` at 717 (720), `keese_killed` at 779 (782). (Without `Obj_Syokudai` registered they're
  the baseline's bytes.)
- **`deku_baba`** and every other case: the same bytes.
- **New case `shutter`:** the exit run, the same bytes over two runs.

Re-recorded and logged in [golden/README.md](../golden/README.md): 89 hashes, 65 cases.

### Decisions

- **[ADR 0038](adr/0038-sliding-doors-and-room-travel.md):** `Door_Shutter` whole with Player's
  sliding door; the door walks skip walls as in the C; the room's temporary clear on its last
  enemy; the interact flags in a `Cell` on the engine's bg actor; the key counter and the lock;
  the debug starts; the exit's run.
- **[ADR 0039](adr/0039-quakes.md):** `z_quake.c` whole, its table a code static on the play state;
  the shake as offsets on the camera; the callers wired.
- **[ADR 0040](adr/0040-switches-torches-webs-and-the-map-data.md):** each actor whole, its later
  triggers injected in the tests; the shared web header; a bake per swapped texture; the torch's
  unshifted `torchType` (`@bug (game)`); the map's state and tables; pack format 20.
- **The ports ran in parallel** as five worktree agents (the switches, the torches and Navi's tags,
  the webs, the quakes, the map), with the doors here; the shared plumbing (the interact flags)
  was written first and handed to each.
- **The exit's run plays the room as it is:** the Keese's dive and Navi's hint are part of it, not
  removed; the run re-lines Link up at the door if he's knocked away, and reads the text.

### Known gaps

- **Not usable by hand yet:** the timed torches and most webs need a burning Deku Stick (4b); the
  eye switches and room 2's ladder need the slingshot (milestone 5), so rooms 2 and 4 to 8 are
  reached by debug starts only.
- **Placeholders still in the Deku Tree:** `Bg_Ydan_Hasi`'s init and kinds, `Bg_Ydan_Maruta`,
  `Obj_Kibako2` (room 0's Gold Skulltula is in the open), `Obj_Lift` (4b); `Obj_Timeblock`,
  `Obj_Bombiwa`, `Obj_Makeoshihiki`, `Bg_Haka` (4c); `Obj_Ice_Poly`. So room 10's floor switch
  raises nothing yet.
- **Nothing of the map is drawn** (milestone 5's pause menu, the HUD's minimap).
- **Quakes:** the roll part isn't drawn (the renderer keeps Y up), nor is the shake applied to the
  prerendered backgrounds or the skybox. No rumble anywhere.
- **The Gohma slab** shakes the main camera, not `Boss_Goma`'s sub camera (milestone 6). *(Fixed in
  6a: her sub camera.)*
- The torches' glow (`Lights_GlowCheck`) and the actors' cull zones aren't ported.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-mechanics.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-shutter.bat
```

### By hand

`game-dungeon.bat WHERE` (or menu 57). WASD the stick, Q is Z, E is B, R the shield, Space A,
I is C-Up (Navi).
1. **`switch`** (the default): walk onto the floor switch in front of you. It sinks with a click
   and a chime; the camera turns to the switch, then to each of the three golden torches as they
   light, and the web over the door to your left (across the floor, by the torch) burns away in
   flames. Then walk to that door: the Keese above it dives at you (R blocks it). In front of the
   door Navi speaks up; read it. Face the door and press Space: it slides up, the camera changes,
   Link walks through; behind him it slams down with dust and a shake, metal bars drop over it,
   and Link starts back. Room 10 is beyond: kill its Deku Baba and the larvae (walk near the eggs
   to hatch them); when the last one dies, the camera turns to the door, the bars lift, and the
   big chest appears (the slingshot).
2. **`lobby-top`**, then jump down the middle: you land on the big web on the ground floor, which
   sags and tears, and you drop through into the basement (room 3).
3. **`lobby`**: walk across the web in the middle of the ground floor: it bounces under you with a
   creak.
4. **`room3`**: one floor switch (on the raised floor at the back) lights the room's golden torch,
   with the camera turning to it; the other (by the torch) drops a small chest from above (a Deku
   Shield) with its own camera.
5. **`room5`** and **`room7`**: stand on the floor switch: the golden torches light while you stand
   there; step off and they go out.
6. **`room9`**: the hint scrubs (as in 3b). Solved, the camera shows the door to room 11 unbarring;
   go through it and you drop into Gohma's room (she's a placeholder).
7. Anywhere: roll into a wall: the camera gives a small shake. Navi now speaks up at her MQ hint
   spots by herself; at the others (room 4's torches, for one) Z-target the spot and press I
   (C-Up).
- **What to report:** whether the door's timing feels right (how close you must stand, how fast it
  opens and slams, Link's pause), the switch's press and the attention cameras' pacing, the web's
  bounce and tearing, the torches' flames and light, and whether the shakes look right.

## Milestone 4b: the Deku Stick and the props

**Answer:** done. The Deku Stick comes out from C-Left, lights at a torch, burns for 210 frames,
lights the timed torches, burns the webs, and breaks on a hit; room 5's log and floating block,
room 10's rising platforms, room 2's lift and ladder, and the crates are ported whole, with the
fragments they break into (`Effect_Ss_Kakera`), which the bushes, rocks and the training boulder
now spawn too. The exit holds; its run is the golden `stick`.

The pack is format 21, in `out/data18`. Decisions are in
[ADR 0041](adr/0041-the-deku-stick-and-the-item-buttons.md) (the stick and the item buttons) and
[ADR 0042](adr/0042-the-deku-trees-props-and-the-fragments.md) (the props and the fragments).

**Scripts** (`scripts\run`, also in `menu.bat`, 59 to 61):
- `test-sticks.bat`: the milestone's tests;
- `game-sticks.bat WHERE`: the game with ten Deku Sticks on C-Left, from a debug start: `torch`
  (room 0's middle floor by its lit golden torch, the default), `room3` (by its lit golden torch,
  the door to room 4 open), `room10` (by its wooden torch), `room5` or `room2` (on the lift);
- `sandbox-stick.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** agreed (2026-10-06). The user chose:
- `Player_UseItem` and `Player_InitItemAction` ported whole: the branches for items no button can
  hold yet (nuts, the lens, spells, masks, the ocarina and bottles, explosives, the hookshot, the
  bow and slingshot) keep their checks and log where they'd start something unported;
- the C's water (corrected after the user's hand test, see "Fixes after playing by hand"): in
  water B and the C buttons are disabled, which puts the stick out;
- milestone 3b's `gold` test breaks room 0's crate with an injected explosion first.

### The survey

**Player, for the stick** (`z_player.c`, `z_player_lib.c`; what the port has today):

| C | What it does for the stick | Port today |
|---|---|---|
| `Player_ProcessItemButtons`, `Player_GetItemOnButton`, `Player_ItemIsInUse` (~70 lines) | B and the three C buttons (`sItemButtons`); a held item no button has is put away | B only |
| `Player_UseItem` (~95) | No sticks left: `NA_SE_SY_ERROR`. Otherwise the change animation or `Player_InitItemActionWithAnim`. Its other branches: the lens, nuts, spells, masks, the ocarina and bottles, explosives | The sword's path only |
| `Player_InitItemAction`, `sItemActionInitFuncs` (~80) | Zeroes `unk_85C`, `unk_858`, `unk_860`; `Player_InitDekuStickIA` sets `unk_85C` (the stick's length) to 1. The others: the bow and slingshot, explosives (spawns `En_Bom`), the hookshot (spawns `Arms_Hook`), the boomerang | Zeroes `unk_860`; no init funcs |
| `Player_FinishItemChange`, `func_8008F2BC` | `NA_SE_PL_CHANGE_ARMS` for a stick | To check |
| `func_80837818`, `Player_CanSpinAttack`, `Player_ActionHandler_8` | The stick always does `FORWARD_SLASH_1H`, never spins, never charges | The stick's lines missing; the hammer's branch and the two-handed `++` too |
| `func_80837948`, `D_80854488` | `DMG_DEKU_STICK` (`ATELEM_SFX_WOOD`), the jump attack's `DMG_JUMP_MASTER` | Ported |
| `func_80842DF4`, `func_80842D20`, `func_80842CF0`, `func_80842AC4`, `func_80842A88`, `func_80842B7C` | A hit or a wall breaks a stick longer than half: `EffectSsStick_Spawn`, `unk_85C` 0.5, `Inventory_ChangeAmmo(-1)`, put away, `NA_SE_IT_WOODSTICK_BROKEN`. (`func_80842B7C`: the Biggoron's Sword's wear) | Logged as not held |
| `Player_UpdateBurningDekuStick` (~25), from `Player_UpdateCommon` | 210 frames: the flame grows over the first 10, `unk_85C` shrinks over the last 20, then -1 stick and put away. The flame is `func_8002836C`'s dust at the tip each frame (scale up to 200, 8 frames) | Not ported (`Effect_Ss_Dust` is) |
| `Player_PostLimbDrawGameplay`, left hand (~25) | The tip `unk_85C × 5000` along the hand, always tracked (`meleeWeaponInfo[0]`); `gLinkChildLinkDekuStickDL`, scaled by `unk_85C` along its length | The sword's half only |
| `Effect_Ss_Stick` (88) | The broken half flying back (child: the stick; adult: the broken Giant's Knife blade) | Not ported |

The data side is already in the pack: `PLAYER_MODELGROUP_10` (closed hands, `SHEATH_18`) is baked
with the other Link variants, and the change tables (`sItemChangeTypes`, `sItemChangeInfo`) are
read whole. `Item_Give` for sticks, `Inventory_ChangeAmmo`, the C buttons' icons and ammo counts
on the HUD, `C_BTN_ITEM`, and J as C-Left are ported. `Obj_Syokudai` (which lights the stick) and
`Bg_Ydan_Sp` (which it burns) read it already.

**Water putting a stick out.** (Corrected after playing by hand: the survey first read the C as
having no such path.) `unk_860` is zeroed only by `Player_InitItemAction`, but in water
`func_80083108` disables B and the C buttons (`Player_GetEnvironmentalHazard`'s
`PLAYER_ENV_HAZARD_SWIMMING` lies between `_UNDERWATER_FLOOR` and `_UNDERWATER_FREE`), and the swim's
actions run the item code: the stick, on no button, is put away and so put out. A press while
standing still (`Player_ActionHandler_Roll`) puts it away too, as does a hit that breaks it.

**The props** (placements from `ootx scene-info --scene ydan`, pack format 20):

| Actor | Placed | C lines | What it needs that isn't ported |
|---|---|---|---|
| `Bg_Ydan_Hasi` | Room 5: `0xFF00` (the floating block), `0xFF01` (the water, on switch 0x3F, which nothing in MQ sets: "never runs in Master Quest"). Room 10: `0x3D02` (the three rising platforms, switch 0x3D, 260 frames up) | 202 | Writes the scene's `waterBoxes[1].ySurface` (the engine's water boxes are read-only); one-point 3040 (ported) |
| `Bg_Ydan_Maruta` | Room 5: `0x00FF` (the spiked log, spinning in place; its tris knock Link back, no damage). Room 2: `0x0121` (the ladder, 280 up, falls on a seed: `DMG_SLINGSHOT`, switch 0x21, one-point 3010) | 218 | Nothing |
| `Obj_Kibako2` | Room 0's middle floor (`0xFFFF`, the Gold Skulltula `En_Sw` `0x8102` inside it); room 10 ×2 | 189 | `Effect_Ss_Kakera`, `func_80033480` (dust puffs), `func_80033684` (an explosion's reach). Broken only by `0x40000040` (the hammer, explosions). Each drops a green rupee (`home.rot.x` 0) |
| `Obj_Lift` | Room 2 (`0x0080`): shakes 20 frames when stood on, falls, breaks into 4 pieces, sets switch 0x20 | 240 | `Effect_Ss_Kakera`, `func_80033480`, `BgCheck_EntityRaycastDown4`; quakes are in |
| `Effect_Ss_Kakera` | Spawned by the crates, the lift, `En_Kusa` (a cut bush's leaves), `En_Ishi` (a broken rock), `En_Goroiwa` (the training boulder at its path's end) | 436 | Bakes per fragment list; `BgCheck_SphVsFirstPoly` |

`En_Goroiwa` is a third caller already ported: today it draws `Rand` as `Effect_Ss_Kakera` and
`func_80033480` would, without spawning them. `En_Kusa` draws none, so the Kokiri Forest runs will
change; `En_Ishi`'s pieces only come from a hammer, an explosion or a throw, none of which happen.

**Elsewhere:** the room 0 Gold Skulltula's crate is a real wall now, so milestone 3b's `gold` test
and hand test (two slashes on that Gold Skulltula) need the crate broken first, by an injected
explosion.

### Scope

- **The Deku Stick from C-Left**, Player's functions above ported whole for the stick. A new save
  preset, `deku-tree-sticks`: `deku-tree-inside` and ten Deku Sticks
  (`Item_Give(ITEM_DEKU_STICKS_10)`) on C-Left. (Switch 0x27 is a temporary flag no save holds:
  the debug starts set it after `Play_Init` instead, the game's new `--switch`.) The Start
  stand-in also puts owned sticks on an empty C-Left.
- **`Effect_Ss_Stick`** and **`Effect_Ss_Kakera`** whole, with `func_80033480` and `func_80033684`;
  `En_Kusa`'s, `En_Ishi`'s and `En_Goroiwa`'s pieces.
- **`Bg_Ydan_Hasi`**, **`Bg_Ydan_Maruta`**, **`Obj_Kibako2`**, **`Obj_Lift`** whole; the sandbox's
  platform rebuilt on the real init; the scene's water boxes writable.
- **Pack format 21** (`out/data18`): the stick's and the broken blade's lists, the fragments'
  lists, the props' bakes.

### The exit

From a debug start on room 0's middle floor by the golden torch (`deku-tree-sticks`, 0x27 set),
Link takes a Deku Stick out (C-Left), holds it to the flame (it catches: `unk_860` 210), runs round
to room 1's door, burns its web (`Bg_Ydan_Sp` 0x1FD6, one-point 3020), and goes through the sliding
door into room 1. That run is the golden. C-derived tests cover room 4's two timed torches opening
its door, room 10's timed torch dropping its chest, a stick breaking on a hit (a target and a
wall), burning down, and each prop.

### What was built

**The Deku Stick** (`oot_actors::player`, `oot_game::effect::stick`, ADR 0041):
- Player's item buttons whole: `Player_ProcessItemButtons` (B and the three C buttons,
  `Player_GetItemOnButton`, a held item no button has put away), `Player_UseItem` (no sticks left:
  `NA_SE_SY_ERROR`; the change or the item at once; the item in hand used), `Player_UpdateItems`
  with the C's conditions (the main camera, no cutscene), `Player_CanUpdateItems`,
  `Player_InitItemAction` with its init functions (`Player_InitDekuStickIA`: `unk_85C` 1), the
  change animation's start, swap and end, and `Player_FinishItemChange`'s sounds
  (`NA_SE_IT_SWORD_PUTAWAY`, `_PICKOUT`, `NA_SE_PL_CHANGE_ARMS`). The branches for items no button
  holds yet (nuts, the lens, spells, masks, the ocarina, bottles, bombs, the hookshot, the bow)
  keep their checks and log what they'd start.
- Swung with C-Left again: always `FORWARD_SLASH_2H` (`func_80837818`; the stick counts as
  two-handed), `DMG_DEKU_STICK` with the wood's sound, no spin. B with the stick out takes the
  sword out instead (B is the sword's), as the C does.
- Broken on a hit or a wall (`func_80842AC4`): the far half flies off backwards
  (`Effect_Ss_Stick`), half stays in hand as it's put away, one stick less,
  `NA_SE_IT_WOODSTICK_BROKEN`. The Biggoron's Sword's wear (`func_80842B7C`) is ported with it.
- Lit at a torch and burning (`Player_UpdateBurningDekuStick`): 210 frames, the flame (a dust puff
  at the tip each frame) growing over the first 10, the stick shrinking over the last 20, then one
  stick less and put away. The tip is tracked every frame (`Player_PostLimbDrawGameplay`), so the
  torches light it and the webs burn from it; the stick is drawn in the left hand, stretched by
  its length.
- In water: B and the C buttons are disabled (`Player_GetEnvironmentalHazard` and
  `func_80083108`'s water branch, ported after the first hand test), so the first swimming frame
  puts the stick away and out, and the sword away.
- The Start stand-in puts owned sticks on an empty C-Left; the preset `deku-tree-sticks`; the
  game's `--switch` debug option.

**The props** (ADR 0042):
- `Bg_Ydan_Hasi` (`bg_ydan_hasi`) whole: room 5's floating block (sliding ±165, bobbing) and its
  water (lowered 5 at init; on its flag, never set in MQ, down 47 for 600 frames and back),
  written into the scene's water box 1, which Link's depth follows; room 10's three platforms,
  undrawn until switch 0x3D, rising 120 with one-point 3040, down after 260 frames, the switch
  popping back up. The sandbox's platform uses the real init.
- `Bg_Ydan_Maruta` (`bg_ydan_maruta`) whole: room 5's spiked log spinning (0x360 a frame), its
  triangles hurting Link a quarter heart and knocking him down along its facing; room 2's ladder,
  280 up until a seed hits it (0x21, the chime, one-point 3010), shaking 20 frames and falling.
- `Obj_Kibako2` (`obj_kibako2`) whole: solid, broken only by the hammer or an explosion (16
  fragments, dust, `NA_SE_EV_WOODBOX_BREAK`), a green rupee, `En_Sw` spawned for params without bit
  15. Room 0's Gold Skulltula is placed inside its crate.
- `Obj_Lift` (`obj_lift`) whole: room 2's platform waits for Link on top, shakes 20 frames with a
  quake, falls, and breaks into 9 pieces with dust on the floor below, setting 0x20 (gone for good).
- `Effect_Ss_Kakera` (`oot_game::effect::kakera`) whole, with `func_80033480` (the dust puffs),
  `func_80033684` (an explosive in reach) and `BgCheck_SphVsFirstPoly`; `En_Kusa`'s leaves,
  `En_Ishi`'s pieces and dust, and `En_Goroiwa`'s pieces are real now.

### Results

**Tests.** `cargo test --release --workspace` (`target/game19`, `OOT_DATA_DIR=out/data18`): 543
passed, 0 failed, 1 ignored (546 with the water fix below). 42 are new, with their expectations
from the C:
- **`oot_actors --test stick`** (10): C-Left takes a stick out frame by frame (the change,
  backwards at 2.4 a frame doubled for an item, the swap on frame 6, `NA_SE_PL_CHANGE_ARMS`, model
  group 10, the tip 66 up, the stick drawn) and A puts it away; none left: the error; B takes the
  sword out; Start puts sticks on C-Left; lit at room 0's torch and burnt down frame by frame (the
  timer, the flame's scale, the length, one stick less, put away); broken on a target (the swing,
  its damage, the half flying at 26 up, 6 back) and on a wall; room 4's timed torches lit with a
  burning stick opening the door to room 5; room 10's wooden torch lighting it and the timed torch
  dropping the chest; and in water (after the fix below): wading in disables the buttons and puts
  the lit stick out the next frame, the sword goes away in the water and the buttons come back on
  land, falling into room 5's pool puts the stick out. `oot_game`'s interface tests: the water
  branch (the hookshot kept on the floor).
- **`--test stick_run`** (1): the exit run.
- **`--test hasi`** (5), **`--test maruta`** (5), **`--test crates`** (5), **`--test lift`** (4),
  **`--test fragments`** (9): each prop and the fragments frame by frame, against replayed `Rand`
  where they draw it; `platform` (one more), `eng_collision` (a written water box).
- **`--test debug_starts`** (one more): the stick's starts stand in their rooms.
- Older tests changed where the new code meets them, each the game's behaviour:
  - `torches`: its stick test takes a real stick out (C-Left) instead of setting one for a frame
    (the real Player puts an item no button has away); Player's update now counts a set
    `unk_860` down before the torch reads it (204, not 205);
  - `skulltulas`: the Gold Skulltula test breaks room 0's crate first with an injected explosion
    (the user's choice).

**The exit run** (`Route::Stick`, `--script stick`, from the debug start by the torch): the stick
out (`stick_out` 28), lit at the torch (`stick_lit` 51; Navi's hint there read with it held to the
flame), round the floor and over the gap at a run, the web burnt from the tip (one-point 3020; the
stick burns out as it does: `web_burnt` 374), Navi's hint at the door, through the door
(`door_opened` 611) into room 1 (`through_door` 647). 648 frames.

**The goldens.** Against milestone 4a's build (`target/game18`, 1ac8d49, on data17), each
difference proven by taking its cause out:
- **`playthrough`**: a cut bush's leaves (`EnKusa_SpawnFragments`) draw `Rand` before its drop, so
  the first bush now drops (from frame 864): `bush` 889 (1084), `deku_tree` 2034 (2236). Without
  the leaves it's the baseline's bytes. With the water fix, the sword in hand is put away in the
  stream Link swims across, so he climbs out with nothing in hand (`anim` from frame 1109); the
  steps are the same frames.
- **`mido_shop_audio`**: `NA_SE_IT_SWORD_PICKOUT` when the sword comes out (frame 2834); the audio
  library's own random then moves which footsteps get their metal clink (`func_800F4010`), the
  same count of sounds. Without the pickout it's the baseline's bytes.
- **`course_platform`** (trace and sheet): the floating block's slide computed in double as the
  C's `M_PI` makes it, about 1e-4 from frame 13. In f32 it's the baseline's bytes.
- Every other case the same bytes. **New case `stick`**: the exit run, the same bytes over two runs.

Re-recorded and logged in [golden/README.md](../golden/README.md): 90 hashes, 66 cases.

### Decisions

- **[ADR 0041](adr/0041-the-deku-stick-and-the-item-buttons.md):** Player's item functions whole,
  the branches for items no button holds logging what they'd start; `heldItemId` an item; Player's
  view of the ammo; the C's water and B; the stick drawn and `Effect_Ss_Stick`; the C-Left stand-in
  and preset; `--switch`; the exit's run.
- **[ADR 0042](adr/0042-the-deku-trees-props-and-the-fragments.md):** each prop and
  `Effect_Ss_Kakera` whole, triggers injected; water boxes written in place; the Kakera tables
  read past their ends as the game does; a bake per fragment list; pack format 21.
- **The ports ran in parallel** as two worktree agents (the fragments with the crates and the lift;
  `Bg_Ydan_Hasi` with the water boxes and `Bg_Ydan_Maruta`), with the stick here.
- **The stick is swung with C-Left**, not B: that's the C (the user's "swung as a weapon" holds,
  from its own button).

### Known gaps

- **The C buttons' other items** (nuts, the slingshot, bombs...) and the pause menu: milestone 5.
  Their branches in `Player_UseItem` log.
- **Not breakable by hand yet:** the crates and rocks (the hammer, bombs), room 2's ladder (the
  slingshot). Room 0's Gold Skulltula sits in its crate, but the sword reaches it through the
  crate: hits aren't blocked by background collision, in the C either.
- **Room 5's water flag** (0x3F) is never set in MQ: its sinking is reachable by injection only.
- **The hazards' other users:** the hot room's and underwater timers (`sEnvHazard`) and the lens's
  magic aren't ported; the C-Up prompt (which dims under water) isn't drawn.
- `Player_ActionHandler_8` (the spin attack's charge) is still not ported (the stick doesn't use it).

### Fixes after playing by hand

- **The lit stick didn't go out in water.** The survey had read the C as having no path for it.
  It has one through the interface: `func_80083108` disables B and the C buttons for
  `Player_GetEnvironmentalHazard`'s values from `UNDERWATER_FLOOR` (2) to `UNDERWATER_FREE` (4), and
  swimming is 3, between them; the port had no hazard. The swim's actions run the item code
  (`Player_TryActionHandlerList` with the upper body), so the first frame Link swims, the stick,
  on no button, is put away and out; the sword goes away too. `Player_GetEnvironmentalHazard` is
  ported whole (with its texts) and the interface's water branch with `sEnvHazard`; the
  `playthrough` golden changed (Link climbs out of the stream with his sword put away).
- **Rolling into a crate doesn't break it:** as in the game. The Deku Tree's crates are the large
  ones (`Obj_Kibako2`), broken only by the hammer or an explosion; the small ones a roll breaks
  (`Obj_Kibako`) aren't placed there.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-sticks.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-stick.bat
```

### By hand

`game-sticks.bat WHERE` (or menu 60). WASD the stick, J is C-Left, E is B, Q is Z, R the shield,
Space A, I is C-Up (Navi), Enter Start.
1. **`torch`** (the default): J takes a stick out (Link draws it from behind, with a sound; the
   C-Left icon shows 10). Walk into the torch in front of you until the stick's end is in the
   flame: it catches (a whoosh), and burns with a growing flame. Navi's hint there opens; read it.
   Turn left and run round the floor: it rises to a ledge; keep running and Link jumps the gap.
   Follow it round to the door with the web; walk into the web with the burning end: the camera
   turns to it and it burns away. If the stick burns out on the way (about 10 seconds), the last
   second it shrinks and Link puts it away; go back for another. Open the door (Space).
2. Anywhere with the stick out: **J again** swings it; hit a wall or an enemy and it breaks (half
   flies off behind, one stick less). **E** takes the sword out instead. **Space** standing still
   puts it away. With none left J gives the error sound.
3. **`room3`**: light the stick at the torch, go through the door ahead (it's open; it bars behind
   you, the Mad Scrub's alive), and touch both torches with the burning end before the first goes
   out: the camera shows the door to room 5 unbarring.
4. **`room10`**: light the stick at the wooden torch by you, drop down to the timed torch below
   and light it: a chest falls with its camera. Step on the floor switch: the camera shows three
   platforms rising by the timed torch; climb them before they sink (about 13 seconds).
5. **`room5`**: the pool has moving water, the block slides and bobs, the spiked log spins with a
   rolling sound. Ride the block into the log: a quarter heart and a knockdown.
6. **`room2`**: you stand on the lift: it shakes (a small camera shake, rattles), falls and breaks
   into planks with dust. Leave the room and come back: it's gone.
7. **Kokiri Forest** (`game.bat`): cut a bush: leaves fly up and fall.
8. **Water** (`room5`, or Kokiri Forest's stream): walk in with a burning stick: as Link starts
   swimming the B and C buttons dim and the stick goes out (put away); a sword in hand is put away
   too. Out of the water the buttons come back.
9. **Start** puts sticks on an empty C-Left: `game-dungeon.bat room10` (no sticks), kill the
   Deku Baba, pick up the stick it leaves, press Enter: the stick appears on C-Left (J).
- **What to report:** how taking the stick out and swinging it feel (C-Left, J twice), the
  flame's size and the stick's burning down, whether the gap jump is fair, the web's burning, how
  the leaves, planks and crate pieces look and move, the platforms' and lift's timing, the log's
  knockdown.

## Milestone 4c: pushing and Master Quest's extras

**Answer:** done. Link holds on to a block and pushes or pulls it; room 3's block goes along the
upper floor's channel and off its end into the pit, where it's the step back up; the gravestones,
the Song of Time's blocks and the rocks are ported whole, their song and explosions injected; one
test walks every connection milestone 4 opened from one start. The exit holds; its run is the
golden `push`.

The pack is format 22, in `out/data19`. Decisions are in [ADR 0043](adr/0043-push-and-pull.md)
(push and pull, the blocks) and [ADR 0044](adr/0044-master-quests-extras-and-room-travel.md) (the
extras, room travel).

**Scripts** (`scripts\run`, also in `menu.bat`, 62 to 64):
- `test-push.bat`: the milestone's tests;
- `game-push.bat WHERE`: the game from a debug start: `room3` (room 3's upper floor behind the
  push block, the default), `room7` (by the gravestones and the hidden stair), `room2` (under the
  rocks' ledge, by the hidden blocks);
- `sandbox-push.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** milestone 4's split, unchanged (surveyed at the end of 4b's session). The
user chose (2026-10-07) to port `Player_ActionHandler_5`'s `Bg_Heavy_Block` branch to its checks,
logging past them that Player's lift isn't ported.

### The survey

**Player, push and pull** (`z_player.c`; the port has none of it: `Player_ActionHandler_5` shows
"Grab" (`PLAYER_STATE2_0`) at a pushable wall and logs on A):
- `Player_ActionHandler_5`'s push branch (A at a wall with `WALL_FLAG_6`, grounded, the wall 39 or
  more high): `unk_3C4` the wall's DynaPoly actor, `func_8083F72C` (`gPlayerAnim_link_normal_push_wait`);
- `func_8083F9D0` (still at that wall with A or `PLAYER_STATE2_4`, else `push_wait_end`),
  `func_8083FAB8` and `func_8083FB14` (into pushing and pulling), `func_8083FFB8` (the stick's
  push or pull along the wall);
- the actions: `Player_Action_8084B78C` (waiting at the wall), `Player_Action_8084B898` (pushing:
  `link_normal_pushing`, `NA_SE_VO_LI_PUSH` on frame 11, the floor's slip sounds, 2 a frame),
  `Player_Action_8084B9E4` (pulling: `PLAYER_ANIMGROUP_pulling`, a floor 26 up and 40 behind checked
  with `func_8083973C` and a line test, -2 a frame);
- `func_8084B840` and `z_actor.c`'s `func_8002DFA4`: the force on the wall's actor
  (`DynaPolyActor.unk_150` += force, `unk_158` the yaw). The engine's bg actor needs `unk_150`,
  `unk_154`, `unk_158` (next to the interact flags of ADR 0038);
- `func_8083F524`; `Player_GetStrength` (`z_player_lib.c`) for the heavier blocks;
- `CAM_MODE_PUSH_PULL` is already requested (`Player_UpdateCamAndSeqModes`) and runs
  `Camera_Parallel1`, ported, on the settings' `PushPull` data.

About 200 lines of C. `PLAYER_STATE2_4` is cleared by `func_80832440` (ported).

**The actors:**

| Actor | Placed (MQ) | C lines | Notes |
|---|---|---|---|
| `Obj_Makeoshihiki` | room 3, `0xFF10`, `home.rot.z` 1 | 144 | Spawns `Obj_Oshihiki` as its child at `sBlocks[1]`'s position for its flags: on the upper floor (-605, -820, -290), or, flag 0x10 set, down in the pit (-365, -905, -290); a small block (`PUSHBLOCK_SMALL_START_ON`). Its **draw** sets and unsets the flags (`sFlagSwitchFuncs`) when the block rests at a position: draw-time state, to run in `draw_update` (ADR 0037's rule). *(Built: the "chime" is `NA_SE_SY_TRE_BOX_APPEAR`; the draw also clears the second flag, 0x3F, though its params turn it off, as the C does.)* |
| `Obj_Oshihiki` | spawned by the above | 688 | The push block: pushed or pulled a block length (`unk_150`), stacking, falling off ledges (`BgCheck_EntityRaycastDown6`, `BgCheck_EntityLineTest3`), `DynaPolyActor_SetSwitchPressed`/`SetActorOnTop`, `Player_GetStrength` for the large kinds, `NA_SE_EV_ROCK_SLIDE`; its textures by colour |
| `Bg_Haka` | room 7 ×8, params 0 | 174 | A gravestone pulled 60 back (`minVelocityY`), `NA_SE_EV_ROCK_SLIDE`; the chime for params 1; `En_Poh` spawned only in the graveyard at night (not ported: a placeholder, never reached here). *(Built: MQ's stones are sunk 15 into the floor, too low for Link to hold on to: see below.)* |
| `Obj_Timeblock` | room 2 ×4, room 7 ×5 (`0x39FF`), room 5 ×1 (`0xB9FF`, on the purple rupee's chest) | 367 | Shown or hidden with its collision from its switch flag (0x3F) *(corrected: MQ's params set bit 6, so it's params bit 15, which the song flips; the flag is unused)*; the Song of Time (`msgCtx.lastPlayedSong`, `ocarinaMode`: not ported, add the fields and inject) toggles it with `Demo_Effect` (a placeholder, as agreed) and an attention camera |
| `Obj_Bombiwa` | room 2 ×3 (params 3, 4, 8) | 158 | Broken by an explosion (`func_80033684`, ported in 4b) or the hammer: `Effect_Ss_Kakera` (ported), `func_80033480`, its switch flag |

**Room travel:** a C-derived test walking the connections milestone 4 opened from one start: 0 to
10 (the switch's web, the door), 0 to 1 (a stick from the middle floor's torch, the web, the door),
the drops 0 to 3 and 3 to 9, then 9 to 11 once the hint scrubs clear room 9.

**Exit (as agreed):** in room 3, Link pushes the block off the upper floor into the pit (flag 0x10
set by `Obj_Makeoshihiki`'s draw, the chime) and climbs onto it. That run is the golden.

### What was built

**Player, push and pull** (`oot_actors::player`, ADR 0043):
- `Player_ActionHandler_5`'s push branch: A at a `WALL_FLAG_6` wall 39 or more high, Link holding
  on (`func_8083F72C`, after putting away what's in hand: `func_8083A388`), `unk_3C4` the wall's
  DynaPoly actor; a `Bg_Heavy_Block`'s strength check (`Player_GetStrength`), its lift logged.
- The actions: holding on (`Player_Action_8084B78C`), pushing (`_8084B898`: push_start, pushing,
  `NA_SE_VO_LI_PUSH` on frame 11, the floor's slips, 2 a frame on the block and Link moving at 2),
  pulling (`_8084B9E4`: the pull group's animations, the floor 40 behind within 20 and no wall
  between, -2 a frame); letting go (`func_8083F9D0`: `push_wait_end`); the stick along Link's
  facing (`func_8083FFB8`); `PLAYER_STATE2_4`; the push camera (`CAM_MODE_PUSH_PULL`, already in).
- The engine: `DynaPolyActor.unk_150`, `unk_154`, `unk_158` on the bg actor slot
  (`Dyna::func_8002DFA4`, `func_8002DF90`); `DynaPoly_GetActor` (`actor_ctx::dyna_poly_get_actor`,
  every DynaPoly actor giving its bg id); `BgCheck_EntityRaycastDown6` and
  `BgCheck_EntityLineTest3`, which skip the caller's own collision.

**The blocks** (ADR 0043):
- `Obj_Oshihiki` (`obj_oshihiki`) whole: the push (0.5 faster a frame up to 2, 20 a push, a
  10-frame wait, `NA_SE_EV_ROCK_SLIDE`, `NA_SE_EV_BLOCK_BOUND` against a wall), the five floor
  points and the fall off a ledge, the wall check along the push, the strength each size needs, the
  init's flags, a block riding on another (`ObjOshihiki_MoveWithBlockUnder`, in `draw_update`),
  pressing a switch it rests on; `gPushBlockDL` baked per texture with its colour.
- `Obj_Makeoshihiki` (`obj_makeoshihiki`) whole: the block spawned as its child where its flags
  say, and its draw's flags and chime where the block rests (in `draw_update`).

**Master Quest's extras** (ADR 0044, a worktree agent):
- `Bg_Haka` (`bg_haka`) whole: the pull to 60, its earth, the dirt patch's sand, the graveyard's
  and Lake Hylia's paths, the Poe (a placeholder).
- `Obj_Timeblock` (`obj_timeblock`) whole: shown or hidden with its collision, the song (Player's
  `PLAYER_STATE2_24` and the new `msgCtx.lastPlayedSong`, injected), `Demo_Effect` (a placeholder),
  the attention camera; drawn with its colour (a bake).
- `Obj_Bombiwa` (`obj_bombiwa`) whole: broken by an explosion or the hammer, its fragments (one more
  `Effect_Ss_Kakera` list) and dust, its flag.

**Room travel** (`oot_actors --test travel`, a worktree agent): from one `Play_Init`, 0 to 10 and
back, 0 to 1 and back, the drops 0 to 3 and 3 to 9, 9 to 11, each room change the real door or drop.

### Results

**Tests.** `cargo test --release --workspace` (`target/game20`, `OOT_DATA_DIR=out/data19`): 579
passed, 0 failed, 1 ignored (546 before). 33 are new, with their expectations from the C:
- **`oot_actors --test push`** (9): holding on and pushing room 3's block frame by frame (Grab, A,
  `push_wait`, the first frame's refusal, `unk_150` and `unk_158`, `pushDist` 0.5, 1.5, 3 ... 19, 20,
  the sounds, the 10-frame wait, the next push); letting go; pulling it back to its start
  (`NA_SE_EV_BLOCK_BOUND` against the channel's step, then no further); off the channel's end into
  the pit (falling 1.5, 4.5, 9 ... and landing on the 11th frame, flag 0x10, 0x3F cleared, the chime,
  immovable); where the block spawns by its flags (Navi's hint there gone with 0x10); spawned
  blocks' flags and the large one's strength; a block riding on another; room 7's sunk stones too
  low to hold; Link pulling a gravestone flush with the floor.
- **`--test push_run`** (1): the exit run.
- **`--test travel`** (1): room travel, 2942 frames.
- **`--test gravestones`** (7), **`--test timeblocks`** (6), **`--test rocks`** (7): each actor frame by
  frame where it matters, against replayed `Rand` where it draws it, the triggers injected.
- **`--test debug_starts`** (one more): the push starts stand in their rooms.
- `eng_collision` (one more): the force fields, the raycast and line test past an actor's own
  collision.
- One older test changed: `skulltula_st` hides room 5's time block as the song would (the shown
  block under room 5's Skulltula is its floor, so it doesn't drop to Link below, as in the game),
  and its drop check maps the drop table through `func_8001F404` (the block's first frames move
  the `Rand` stream: without the block there's no drop at all, as at the baseline; the time block's
  attention flag was ruled out).

**The exit run** (`Route::Push`, `--script push`, from `PUSH_START` on room 3's upper floor behind
the block): Navi's hint by the block (0x108) read on the way, Link holds on (`block_grabbed` 327),
twelve pushes along the channel and off its end (`block_in_pit` 600: flag 0x10, the chime), out of
the channel, down into the pit south of the block, and up onto it: pressed against its 60-high
side he jumps and grabs its edge, and climbs up with the stick at full tilt (`on_block` 729). 729
frames.

**The goldens.** Against milestone 4b's build (`target/game19`, 6987231, on data18: its own check
90/90 identical), every one of the 90 hashes is the same bytes: no run enters rooms 2, 3, 5 or 7,
and none holds A at a pushable wall. **New case `push`**: the exit run, the same bytes over two runs.
Re-recorded and logged in [golden/README.md](../golden/README.md): 91 hashes, 67 cases.

### Decisions

- **[ADR 0043](adr/0043-push-and-pull.md):** Player's push and pull whole, the heavy block's lift
  logged past its checks (the user's choice); the force on the engine's bg actor; `DynaPoly_GetActor`
  through `dyna_bg_id`; the raycast and line test skipping an actor's own collision; the blocks
  whole, their draws' state in `draw_update`.
- **[ADR 0044](adr/0044-master-quests-extras-and-room-travel.md):** the extras whole, the song
  injected; room 7's stones left as the data has them (too low to hold), Player's pull tested on a
  stone flush with the floor; the Skulltula test's hidden block; the travel test's real doors and
  drops.
- **The ports ran in parallel** as two worktree agents (the extras; the travel test), with Player and
  the blocks here.
- **No "block already in the pit" debug start:** the game's `--switch` sets flags after the room
  loads, and `Obj_Makeoshihiki`'s draw would clear 0x10 again with the block at its first place.

### Known gaps

- **Room 7's gravestones can't be pulled in Master Quest:** sunk 15 into the floor, their fronts
  stand 34 over it (28 where Player measures), under the 39 holding on needs; Link climbs onto them.
  That's the data, not a port gap (checked against `Player_ActionHandler_5`).
- **The ocarina and bombs:** the time blocks and the rocks wait for them (milestone 5 and later);
  `Demo_Effect` and `En_Poh` are placeholders. Player's lift (the heavy block, the pots) isn't
  ported (BACKLOG #4).
- **Found, in the backlog:** Player doesn't ask for `CAM_MODE_STILL` while knocked down (#18); an
  init's children come after their parent in the port's lists, before it in the C's (#19). (#17,
  room 0's middle-floor vines not reaching the top floor, was the travel test's scripted climb: by
  hand they do.)
- The travel test places Link within a room three times (room 0's top floor, room 3's upper floor:
  the slingshot's loop, beside the running hint scrub).

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-push.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-push.bat
```

### By hand

`game-push.bat WHERE` (or menu 63). WASD the stick, Space A, E is B, Q is Z, R the shield, I is
C-Up (Navi), Enter Start.
1. **`room3`** (the default): walk up to the block ahead; Navi calls (read her hint with Space).
   At the block "Grab" shows: hold Space and Link holds on (the camera moves behind him). Still
   holding Space, push W: he heaves (a grunt) and the block slides a block length with a grinding
   sound, then a short pause before the next. Let go of Space: he lets go.
2. **Pull:** after a push or two, hold Space and pull S: he pulls it back towards him, backing off.
   Back at its start it stops with a thud and won't come further.
3. **Into the pit:** push it twelve times to the channel's end: it drops into the pit below with a
   thud and a chime. Try to push it again from above or below: it won't move.
4. **Climb:** step off the floor's edge beside the block (south of it) into the pit (water), turn to
   the block and walk into it: Link jumps up, grabs its edge and climbs on with the stick held
   fully. From its top, the upper floor is a step up.
5. **`room7`**: the gravestones stand low in the floor: walking into one, Link climbs onto it (no
   "Grab"). The Song of Time's stair here is hidden (no ocarina yet). Room 5 (`game-dungeon.bat
   room5`) has the purple block standing on the chest.
6. **`room2`**: the three rocks on the ledge above are solid (the sword bounces off: no bombs yet);
   the four Song of Time blocks around you are hidden.
7. **Vines** (`game-dungeon.bat lobby`, BACKLOG #17): climb room 0's middle-floor vines by the
   golden torch to their top: does Link get onto the top floor, or drop off under its rim? In the
   game, compare.
- **What to report:** how holding on, pushing and pulling feel (the A hold, the pauses, the camera),
  the block's look and sounds, the drop and the chime, the climb onto it from the pit, and the
  vines.

### Played by hand

The user played it (2026-10-07): the blocks work, and room 0's middle-floor vines take Link to the
top floor (BACKLOG #17 closed: only the travel test's scripted climb fell short).

## Milestone 5: items in use

**Status:** split agreed (2026-10-07). The user chose:
- three parts, 5a, 5b and 5c, as below;
- `En_Arrow` ported whole, the adult arrows' trail (`EffectBlure`) and the magic arrows' children
  logging what they'd do;
- 5b's pause menu with the item page and the dungeon map page whole; the equipment and quest
  pages showing their backgrounds, their contents logged; Start's stand-in keeping only its
  equipment half;
- until then, Start's stand-in putting an owned slingshot and nuts on empty C buttons, and a
  preset with both.

### The survey

Line counts are the decomp's (`52a510f`), per function where it matters. "Port today" is
`3f5c0ac` (milestone 4c).

**Player, the slingshot** (`z_player.c`; `sItemActionUpdateFuncs` sends the bow, the slingshot and
the hookshot to `func_8083501C`, which the port maps to `func_8083485C`'s default):

| C | What it does | Lines | Port today |
|---|---|---|---|
| `func_8083501C`, `func_80834F2C`, `func_80834D2C`, `func_80834EB8` | The upper body's wait: a press of the item's button raises it (`gPlayerAnim_link_bow_bow_ready`); Z-targeted it stays third person, else `unk_6AD` 2 (first person) | 66 | Not ported |
| `func_8083442C`, `func_80834380` | Raised: `func_808351D4`, `PLAYER_STATE1_9`, `unk_834` 14; a seed in the pouch spawns `En_Arrow` (`ARROW_SEED`) as Player's child, held (`heldActor`) | 61 | Not ported |
| `func_808351D4`, `func_808350A4` | Drawn back (the upper body's z roll to 1200); the button let go fires: the seed let go (`unk_A73` 4, the parent cleared), one seed less (`Inventory_ChangeAmmo`), a rumble; none: `NA_SE_IT_SLING_FLICK` | 87 | Not ported |
| `func_808353D8`, `func_80835588` | After the shot: the next one on the button (`link_bow_bow_shoot_next`), or lowered (`_shoot_end`) | 52 | Not ported |
| `func_80834FBC`, `func_80834E44`, `func_80834E7C` | The hookshot's hook; the shooting gallery's B (`shootingGalleryStatus` 0 here) | 24 | Not ported (never true for the child) |
| `Player_InitBowOrSlingshotIA` | `PLAYER_STATE1_3`, `unk_860` -2 | 10 | Ported (4b) |

**Player, first person** (C-Up's look and the aim):

| C | What it does | Lines | Port today |
|---|---|---|---|
| `func_8083B8F4` | C-Up on the ground (or swimming shallow): `unk_6AD` 1, if the camera allows `CAM_MODE_FIRST_PERSON` | 13 | Logs (`Player_ActionHandler_0`) |
| `Player_ActionHandler_13`'s first-person branch, `func_8083AD4C` | `unk_6AD` 1 or 2: `CAM_MODE_FIRST_PERSON` or `_AIM_CHILD`; `Player_Action_8084B1D8`, `PLAYER_STATE1_20`, `NA_SE_SY_CAMERA_ZOOM_UP` (the error sound when the camera refuses) | 40 | Logs. The cutscene items' branch (`unk_6AD` 4: the ocarina, bottles, trades, spells) stays logged |
| `Player_Action_8084B1D8`, `func_8084ABD8`, `func_8084AEEC`, `func_8084B000`, `func_8083C148`, `func_8083B010` | The look: the stick turns the focus (slowly for the look, eased for the aim, ±19114 of the body); A, B, R (or any C button in the look) ends it; in water it floats | 151 | Not ported |
| `func_8002DD6C`, `func_8002DD78` (`z_actor.c`) and their callers in ported functions: `func_8083DC54`, `func_8083FC68`, `func_8083FD78`, `func_8083356C` (`link_bow_side_walk`), `Player_UpdateCamAndSeqModes` (`CAM_MODE_Z_AIM`), `Player_UpdateInterface`, `Player_UpdateCommon` (`func_8084FF7C`, the string's swing) | Aiming Z-targeted, in third person | ~60 | The branches are missing (one logs) |

**Player, Deku nuts:** `func_8083C61C` (13: not in an indoors room, on the ground, nuts left) and
`Player_Action_8084E604` (14: `link_normal_light_bom`; frame 3 one nut less and `En_Arrow`
`ARROW_NUT` from the right hand, pitched 4000, `NA_SE_VO_LI_SWORD_N`), and
`Player_UpdateUpperBody`'s return after it. `Player_UseItem` logs today.

**Player's draw** (`z_player_lib.c`): `Player_OverrideLimbDrawGameplayFirstPerson` (25: in the
look nothing is drawn; aiming, the arms and the slingshot, `sFirstPerson*DLs`), chosen by
`Player_Draw` when the head is behind the view (`unk_6AD` set); the right hand's string
(`gLinkChildSlingshotStringDL` stretched by `unk_858` towards the left hand, `PLAYER_STATE1_9`); the
held seed placed by the left hand's matrix (`heldActor`). The `BOW_SLINGSHOT` model group (the
slingshot in the right hand) is already baked as a Link variant; the first-person lists and the
string aren't. Player holds no actor today (`heldActor`; `Player_DetachHeldActor` notes it).

**The camera:** `Camera_Subj3` (137) runs `CAM_MODE_FIRST_PERSON`, `_AIM_CHILD` and `_Z_AIM` on
every setting the Deku Tree uses (`NORMAL0`'s data); not ported. `Camera_RequestModeImpl`'s
first-person sounds and `Camera_UpdateInterface` (the HUD's mode from the data's interface field)
are.

**Actors and effects:**

| C | What | Lines | Needs |
|---|---|---|---|
| `En_Arrow` | Seeds (`ARROW_SEED`: 80 a frame, 15 frames, gravity -0.4 for the last 7, `DMG_SLINGSHOT`, the sparkle `gEffSparklesDL`, `NA_SE_IT_SLING_SHOT`; on a hit or a wall `Effect_Ss_Stone1` and `NA_SE_IT_SLING_REFLECT`) and nuts (`ARROW_NUT`: the same flight; on a hit or a wall `En_M_Fire1`, the screen's flash, `NA_SE_IT_DEKU`). The adult's arrows: `gArrowSkel` and its two animations, sticking into walls and actors, carrying an actor (`ACTOR_FLAG_CAN_ATTACH_TO_ARROW`), the trail (`EffectBlure`), the magic arrows' children | 513 | `BgCheck_ProjectileLineTest`, `func_8002F9EC` (Jabu Jabu's walls), `Player_UpdateWeaponInfo` for an actor's collider (the port's is Player's), `EffectBlure` (`z_eff_blure.c`, 1,056: not ported, the sword's trail debt), `Arrow_Fire`/`_Ice`/`_Light` (not ported) |
| `En_M_Fire1` | The nut's stun: a cylinder of radius 200, `DMG_DEKU_NUT`, for five frames | 79 | Nothing |
| `Effect_Ss_Stone1` | The seed's and nut's puff | 96 | A bake |
| The flash | Play's `transitionFadeFlash` (`TransitionFade`'s flash type, stepped by `R_TRANS_FADE_FLASH_ALPHA_STEP`), grey 160 | ~25 | Not ported (the port has the other fades) |

Already waiting for them: `Obj_Switch`'s eye (`0x0001F824`, injected in its tests), `Bg_Ydan_Maruta`'s
ladder (`DMG_SLINGSHOT`, injected), and the enemies' damage tables' slingshot and nut entries.

**Getting them:** the slingshot is room 10's chest (`En_Box` `0x10A6`: `GI_SLINGSHOT`, flag 6,
appearing when room 10 is cleared: its Deku Baba and Gohma's eggs). `Item_Give` (the bullet bag,
30 seeds) is ported. Nuts come from drops and the Business Scrub (`En_Dns`). Nothing puts an item
on a button but the pause menu: until it's ported, Start's stand-in puts sticks on an empty C-Left
(ADR 0041). The HUD draws the C buttons' items and ammo; K and L are C-Down and C-Right.

**The rooms the slingshot opens** (placements from `ootx scene-info --scene ydan`):
- room 1's eye (0x0C) opens the door to room 2; room 2 is a dead end (the ladder's seed, 0x21; the
  compass chest; the rocks and the hidden time blocks wait for bombs and the song);
- room 3's eye (0x15) opens the door to room 4; then 4 to 5 (room 4's timed torches, 0x19), 5 to 6
  (room 5's timed torches, 0x09), 6 to 7, 7 to 8, and 7 back to room 3's upper floor (the
  `En_Holl` plane). Rooms 4, 6 and 7 bar their doors behind Link until they're cleared: room 4's
  Mad Scrub; room 6's Mad Scrub, Keese and eight Gohma eggs; room 7's two Deku Babas, a withered
  one and four Keese. Clearing them by fighting waits on `Rand`.

**The pause menu** (`ovl_kaleido_scope`, about 9,000 lines with its tables; the roadmap's 7,700):

| File | Lines | What |
|---|---|---|
| `z_kaleido_setup.c` | 200 | `KaleidoSetup_Update` (Start: ported as the stand-in's entry), `KaleidoSetup_Init` |
| `z_kaleido_scope.c` | 4,751 | About 940 of tables; `KaleidoScope_Update` (1,085: every pause state, opening, the pages, the save prompt, closing, the game over's (a stand-in today, ADR 0032), the debug menu's entry); `KaleidoScope_Draw` (48), `_DrawPages` (433: the four pages' box, each page's contents called), `_DrawUIOverlay` (496: the name and info panels, the buttons' prompts), `_SetVertices` and `_SetPageVertices` (828: the pages' quads), the cursor (137), page turns (99), `_UpdateNamePanel` (71), `_UpdateOpening` (32), the dungeon map's load and update (48), `_DrawGameOver` (45), the player's prerender for the equipment page (32) |
| `z_kaleido_item.c` | 869 | The item page: the cursor over the 24 slots (`KaleidoScope_DrawItemSelect`, 415), the ammo digits (60), equipping on a C button with the icon flying there (`KaleidoScope_UpdateItemEquip`, 327) |
| `z_kaleido_equipment.c` | 715 | The equipment page, with Link drawn in it (`KaleidoScope_DrawPlayerWork`) |
| `z_kaleido_map.c` | 945 | The dungeon map page (342: the map and compass milestone 4 left as data, "drawn by milestone 5's pause menu") and the world map (575) |
| `z_lmap_mark.c` | 183 | The chests' and the boss's marks on the dungeon map (their data is in the pack, ADR 0040) |
| `z_kaleido_collect.c` | 873 | The quest status page |
| `z_kaleido_prompt.c` | 45 | The prompts' cursor (ported for the game over stand-in) |

Its textures: `icon_item_static` (the item icons, read for the HUD already, and the pages'
backgrounds), `icon_item_24_static`, `icon_item_<language>_static` (the titles and prompts),
`item_name_static`, `map_name_static`; the paused scene behind it (`z_prerender.c`'s filters). The
game over screens draw through the same `KaleidoScope_Draw`, so they'd be drawn too.

Start's stand-in equips more than items: it puts owned, unworn swords, shields, tunics and boots on
(`SaveContext::equip_owned_unworn`), which the scripted runs from Kokiri Forest rely on. That's the
equipment page's job in the C.

**Saving** (`z_sram.c`, 1,086):
- `Sram_WriteSave` (53: the checksum, the slot and its backup), `Sram_OpenSave` (168: the entrance
  a save loads at by its saved scene, dungeons at their entrances), `Sram_VerifyAndLoadAllSaves`
  (184: the checksums, a bad slot restored from its backup), `Sram_InitSave` (105: a new file's
  name and slot), `Sram_EraseSave`, `Sram_CopySave`, `Sram_InitSram` (55: the header, its "ZELDA"
  check, the sound and Z-targeting options); the iQue's path (163) isn't this ROM.
- Callers: the pause menu's save prompt (`KaleidoScope_Update`), the game over's (ported, logs
  `Sram_WriteSave`).
- The port has `Sram_InitNewSave` and `Sram_InitDebugSave` (ADR 0019) and `Play_SaveSceneFlags`; no
  file on disk. The file select (`z_file_choose.c`) isn't ported, so loading needs a stand-in, for
  example a `--file N` option doing what `FileSelect_LoadGame` does. The `Save` struct (`save.h`)
  would be written in the C's byte layout, so a slot holds the game's bytes.

**Elsewhere:** BACKLOG #18 (no `CAM_MODE_STILL` when knocked down) is in
`Player_UpdateCamAndSeqModes`, the function `CAM_MODE_Z_AIM` goes into: it can ride along with the
slingshot.

### The split

- **5a, the slingshot and Deku nuts** (about 1,500 lines of C, like 4b):
  - Player: the slingshot's upper actions, first person (C-Up's look and the aim), aiming
    Z-targeted, the nut's throw, `heldActor`, the first-person draw and the string (bakes);
    `Camera_Subj3`; BACKLOG #18.
  - `En_Arrow` whole for seeds and nuts, `En_M_Fire1`, `Effect_Ss_Stone1`, the flash,
    `BgCheck_ProjectileLineTest`. The adult's arrows ported too, their trail and the magic arrows
    logging.
  - Equipping before the pause menu: Start's stand-in also puts an owned slingshot and nuts on
    empty C buttons; a preset with both.
  - The eye switches, room 2's ladder and room 10's chest for real (the tests' injections replaced
    where the real shot now reaches them).
  - The travel test extended: room 1 to 2 (the eye) and back; room 3's eye to 4, 5, 6, 7, 8 and
    back to room 3's upper floor through the plane, without its `place_player` there. The rooms
    that bar until cleared (4, 6, 7) and room 10's chest have their enemies killed by injection
    (`Actor_Kill`), not fought.
  - **Exit:** from a debug start in room 1 with the slingshot, Link takes it out, aims in first
    person at the eye and shoots: the eye closes, the door to room 2 unbars with its camera, and he
    goes through. That run is the golden.
- **5b, the pause menu:** its frame (`KaleidoSetup`, the open and close, the pages' box and turns,
  the cursor, the name and info panels) and the item page whole, replacing the item half of
  Start's stand-in; the C buttons equipped from it. The dungeon map page (with its marks) whole too;
  the equipment and quest pages showing their backgrounds, their contents logged. The game over
  screens drawn.
- **5c, saving:** `z_sram.c`'s save and load whole, the slots on disk in the C's layout, the pause
  menu's save prompt, the game over's `Sram_WriteSave`, and a stand-in for the file select's load.

## Milestone 5a: the Fairy Slingshot and Deku nuts

**Answer:** done. C-Right takes the Fairy Slingshot out and raises it into first person; held, it
draws a seed; let go, it shoots, and the seeds close the eye switches and drop room 2's ladder.
Z-targeted it aims in third person; C-Up looks around in first person; C-Down throws a Deku nut,
which flashes the screen and stuns. Room 10's chest gives the slingshot, and Start's stand-in puts
it and nuts on C buttons until 5b's pause menu. The exit holds; its run is the golden `slingshot`.

The pack is format 23, in `out/data20`. Decisions are in
[ADR 0045](adr/0045-the-fairy-slingshot-first-person-and-deku-nuts.md) (Player's side) and
[ADR 0046](adr/0046-en-arrow-the-nuts-stun-and-the-flash.md) (the projectiles and the flash).

**Scripts** (`scripts\run`, also in `menu.bat`, 65 to 67):
- `test-slingshot.bat`: the milestone's tests;
- `game-slingshot.bat WHERE`: the game with the slingshot on C-Right (L), nuts on C-Down (K) and
  sticks on C-Left (J) (`deku-tree-slingshot`), from a debug start: `room1` (250 in front of room
  1's eye switch, the default), `room3` (under room 3's eye switch), `room2` (in front of the
  ladder) or `room10` (by the slingshot's chest);
- `sandbox-slingshot.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** milestone 5's split, as agreed (2026-10-07; see "Milestone 5" above).

### What was built

**Player** (`oot_actors::player`, ADR 0045):
- The bow's and slingshot's upper-body actions whole (`func_8083501C`, `func_80834F2C`,
  `func_80834D2C`, `func_80834EB8`, `func_8083442C`, `func_80834380`, `func_808351D4`,
  `func_808350A4`, `func_808353D8`, `func_80835588`, the hookshot's `func_80834FBC`, the shooting
  gallery's checks); the string's spring (`func_8084FF7C`).
- First person: C-Up's look (`func_8083B8F4`), `Player_ActionHandler_13`'s first-person branch
  with `func_8083AD4C`, `Player_Action_8084B1D8` with `func_8084ABD8` (the stick's turn: steady in
  the look, eased when aiming). The main camera's answers are computed by Player from a view of
  it, the requests made after his update.
- Aiming Z-targeted: the branches in `func_8083DC54`, `func_8083DDC8`, `func_8083FC68`,
  `func_8083FD78`, `func_8083356C` (the bow's side walk) and `CAM_MODE_Z_AIM`.
- `heldActor`: the seed spawned as Player's child (`Actor_SpawnAsChild`) and let go through
  requests, placed by his draw at the left hand; `Player_DetachHeldActor` whole and called where
  the C calls it (`func_80832564`, `func_80834644`, `func_808346C4`, the guard, a cutscene mode).
- The Deku nut: `func_8083C61C` (not indoors, on the ground) and `Player_Action_8084E604` (frame 3:
  one nut less, `En_Arrow` `ARROW_NUT` from the right hand, pitched 4000).
- The draw: the first-person Link variant (`Player_OverrideLimbDrawGameplayFirstPerson`: the
  arms only when aiming, nothing in the look), the string from the right hand
  (`gLinkChildSlingshotStringDL`, stretched by `unk_858`, draw-time state).
- Player's `actor.focus.pos` as the C has it (`sPlayerFocusOffsetFromHead` in the head's drawn
  space; it was the head limb's origin): found by the first person's view (below).
- BACKLOG #18: knocked down, Player asks for `CAM_MODE_STILL`.

**The camera:** `Camera_Subj3` whole (first person, the aims), with Player's focus in the
camera's view of him.

**The projectiles** (ADR 0046, a worktree agent): `En_Arrow` whole (the adult arrows' trail and the
magic arrows logging), `En_M_Fire1`, `Effect_Ss_Stone1`, Play's flash (`transitionFadeFlash`,
`R_TRANS_FADE_FLASH_ALPHA_STEP`), `BgCheck_ProjectileLineTest`, `func_8002F9EC`, and
`Player_UpdateWeaponInfo` for any actor.

**Equipping and starts:** Start's stand-in puts owned nuts and the slingshot on the first empty C
buttons (`SaveContext::equip_item_on_c`, `KaleidoScope_UpdateItemEquip`'s swap for any C button);
the preset `deku-tree-slingshot`; `SLINGSHOT_STARTS` (room 1's eye, room 3's eye, room 2's ladder,
room 10's chest); `Route::start_clears` (a debug start's rooms marked cleared before it loads
them).

### Results

**Tests.** `cargo test --release --workspace` (`target/game21`, `OOT_DATA_DIR=out/data20`):
603 passed, 0 failed, 1 ignored (579 before). 24 are new, with their expectations from the C:
- **`oot_actors --test slingshot`** (12): the slingshot out (the change's swap on its 4th frame,
  `PLAYER_STATE1_3`, `unk_860` -2, `NA_SE_PL_CHANGE_ARMS`), raised into first person the next frame
  with the aim's camera and no seed (the C's dry first raise), then drawn with a seed after the
  ready animation (`NA_SE_IT_SLING_DRAW`, the seed Player's child at the left hand); the shot on
  letting go (29 seeds, `unk_A73` 4, the parent cleared) and the string's spring frame by frame;
  the flick with no seeds; A ending first person and the slingshot lowered
  (`link_bow_bow_shoot_end`); C-Up's look (`unk_6AD` 1, the stick turning the focus 960 a frame);
  the Z-targeted aim (`CAM_MODE_Z_AIM`, no first person); a nut thrown (frame 3, at the right
  hand, pitched 4000, the voice); Start's stand-in and the C buttons' swap; the first-person draw
  (the arms' variant, baked, and the string); `Camera_Subj3`'s settled view (at, eyeNext, eye from
  the data and Player's focus); room 10's chest giving the slingshot (30 seeds, flag 6) and Start
  putting it on C-Down; a seed from the `room2` start over the lift into the ladder.
- **`--test arrows`** (10): a seed let go without a shot is gone at once; a shot seed's 15 frames
  (speed 80, gravity from the 8th, positions and the quad frame by frame); a held seed waits; a
  seed into a wall (the burst's 8 frames, `NA_SE_IT_SLING_REFLECT`); a nut flying whatever
  `unk_A73` (and `ARROW_CS_NUT`); a nut into a wall (the flash's alpha 255, 165, 135 ... 0,
  `En_M_Fire1`, `NA_SE_IT_DEKU`); the nut's stun on room 5's Skulltula; real seeds into room 1's
  eye, room 2's ladder and room 3's eye.
- **`--test slingshot_run`** (1): the exit run.
- **`--test debug_starts`** (one more): the slingshot's starts stand in their rooms.
- `--test damage`: the knockdown asks for `CAM_MODE_STILL` (BACKLOG #18).
- **`--test travel`** (extended, a worktree agent): from the one start, now with the slingshot,
  also room 1 to 2 (room 1's eye shot, room 2's ladder shot from its floor and climbed) and back;
  and instead of placing Link on room 3's upper floor, the loop: room 3's eye, rooms 4 (its timed
  torches lit with a stick from room 3's torch), 5 (its golden torch while its switch is held, the
  floating block ridden under the spiked log crouching with R, its timed torches), 6, 7 (the webs
  over the doors to room 8 and to the crawlspace burnt with sticks lit at its held-switch torches),
  8 and back, the crawlspace to room 3's upper floor; then room 3's block pushed into the trench
  (`Route::Push`) to carry fire from room 3's torch up to its floor web, the drop to room 9 and 9
  to 11 as before. 7943 frames (2942 before). Two `place_player`s are left: room 0's top floor
  (the scripted vine climb, BACKLOG #17's) and beside room 9's running hint scrub.
- One older test changed: `skulltula_st`'s killing slash expected at most one drop, but
  `Item_DropCollectibleRandom` drops its entry's `sDropQuantities`; with the focus fix the `Rand`
  stream (Navi's flight) lands on an entry of more than one. It now checks one entry's item and
  quantity.

**The exit run** (`Route::Slingshot`, `--script slingshot`, from `SLINGSHOT_START` in room 1 with
the room cleared): C-Right held, the slingshot out and drawn in first person (`slingshot_drawn`
19), the stick steering the seed's aim at the eye (from the seed's own position and rotation),
let go: the seed bursts on the eye (`eye_shot` 31, flag 0x0C), A out of first person, the door's
attention cameras, then through the door (`door_opened` 245) into room 2 (`through_door` 281).
281 frames.

**The goldens.** Against milestone 4c's build (`target/game20`, 3f5c0ac, on data19: 91/91
identical): `combat`, `shutter`, `stick`, `mido_shop_audio` and `new_file_deku_tree` changed, each
from Player's focus (the one-point cutscenes' shots aimed at Link, Navi's resting point, the
attention system's line of sight), proven by reverting it; every other case the same bytes. **New
case `slingshot`**: the exit run, the same bytes over two runs. Then, after playing by hand, the
near plane (below): 28 renders changed (the course sheets' posts nearest the camera, the edges of
Kokiri Forest's, Hyrule Field's and the Deku Tree's shots), no trace, proven by taking the feature
out. Logged in [golden/README.md](../golden/README.md): 92 hashes, 68 cases.

### Decisions

- **[ADR 0045](adr/0045-the-fairy-slingshot-first-person-and-deku-nuts.md):** Player's slingshot,
  first person and nut whole, the boomerang's actions noted; the camera's answers computed by
  Player; `heldActor` through requests and the draw; the first-person variant; the stand-in and
  preset; the dry first raise kept; `Camera_Subj3`; BACKLOG #18; Player's focus; the exit's run
  from room 1 cleared.
- **[ADR 0046](adr/0046-en-arrow-the-nuts-stun-and-the-flash.md):** `En_Arrow` whole with the
  trail and magic arrows logging; `En_M_Fire1`, `Effect_Ss_Stone1`, the flash, the projectile line
  test, the shared weapon info.
- **The ports ran in parallel** as two worktree agents (the projectiles; the travel test), with
  Player, the camera and the bakes here.
- **Room travel, from the C** (the agent's findings): room 7's way to room 3 is a crawlspace, which
  puts a stick away (`Player_TryEnteringCrawlspace`), so room 3's floor web takes fire from room
  3's own torch, carried over the trench by the pushed block; door 7 (type 1, sides 7 and 6) is
  barred from room 6, not 7 (`DoorShutter_SetupDoor`: only its back stays `SHUTTER_FRONT_CLEAR`);
  room 5 is crossed on the floating block in its phase, under the spiked log crouched (R: the
  cylinder 38 to 19 high). Enemies killed during a cutscene are deleted only once it's over
  (`sCategoryFreezeMasks`), so a room's clear can come late; the test keeps room 4's far egg until
  its torches are lit.
- **The exit's run starts with room 1 cleared:** its big Deku Baba (scale 2.5) wakes within 500 of
  Link anywhere on the floor and bit him while he aimed (`Rand`'s fight). `game-slingshot.bat
  room1` keeps it.

### Known gaps

- **The pause menu** (5b): Start is still the stand-in, putting owned items on empty C buttons.
- **The adult's arrows' trail** (`EffectBlure`) and the magic arrows' actors log; the boomerang's
  upper-body actions (`func_80835800` on) aren't ported (its item is a later dungeon's).
- **Player keeps its own copy** of `Player_UpdateWeaponInfo` (fields `tip`/`base`) beside the
  shared one; folding it in is left for later.
- `bodyPartsPos` for a limb the first-person draw leaves out: the C keeps the last frame's
  (`sCurBodyPartPos` only moves for a drawn limb); the port computes them all.

### Fixes found while building

- **Player's focus was the head limb's origin**, not the C's `sPlayerFocusOffsetFromHead` in the
  head's drawn space: in first person the camera (`Camera_Subj3`, which hangs off it) sat at the
  neck and the slingshot showed at the top of the screen. Fixed (ADR 0045); five goldens moved.

### Fixes after playing by hand

- **Part of the slingshot's string was missing** as Link drew it back: the string's pouch comes
  nearer to the camera than the near plane. The game's microcode is F3DZEX2's NoN variant
  (`graph.c`: `gspF3DZEX2_NoN_fifo`), which clips nothing at the near plane, only at the far one;
  the GPU clipped both. The renderer now draws with unclipped depth (wgpu's
  `DEPTH_CLIP_CONTROL`, `eng_render::NON_FEATURES`, asked for in the headless and the window's
  devices where the adapter has it; a nearer depth clamped to the near plane) and drops a pixel
  past the far plane in `shader.wgsl` (ADR 0045). Everything near the camera draws as on the
  console: 28 renders changed, no trace (golden/README.md). A GPU without the feature clips as
  before.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-slingshot.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-slingshot.bat
```

### By hand

`game-slingshot.bat WHERE` (or menu 66). WASD the stick, L is C-Right (the slingshot), K C-Down
(nuts), J C-Left (sticks), I C-Up, Space A, E is B, Q is Z, R the shield, Enter Start.
1. **`room1`** (the default): L takes the slingshot out and the view goes into first person (a
   letterbox, the slingshot's fork in front). Keep L held: after a moment the string draws back
   with a sound (the first raise is dry, as in the C). Aim with the stick at the eye over the
   door (pushing up aims down, as in the game; the aim turns slowly, then faster) and let go of
   L: the seed flies with a twang and bursts with a puff on the eye, which closes with a chime;
   the camera shows the door's bars going up. Space (A) leaves first person. Go through the door
   into room 2. Room 1's big Deku Baba in the middle wakes and lunges: fight it or keep your
   distance.
2. In first person, L again shoots the next seed; the C-Right counter counts down. Set the
   seeds aside: at 0 the string only flicks. E (B) leaves first person and takes the sword out.
3. **Z-aim:** Q (Z) then L: the slingshot raised in third person, the camera over the shoulder;
   walk sideways with the stick.
4. **The look:** I (C-Up) with nothing to talk about: first person without the slingshot; the
   stick looks around; I or Space again returns.
5. **Nuts:** K throws a Deku nut: a grey-white flash where it lands, and what's near it is
   stunned (room 1's big Deku Baba, from the `room1` start, freezes for a moment). The nut count
   goes down. A nut flies through enemies and bursts only on walls and floors, as in the game.
6. **`room2`**: shoot the ladder above the door's ledge: it shakes and falls with its camera;
   climb it to the door.
7. **`room3`**: shoot the eye over the door ahead: the door to room 4 opens.
8. **The chest:** `game-sticks.bat room10` (no slingshot yet): kill room 10's enemies; the big
   chest drops with its camera; open it: the Fairy Slingshot. Enter (Start) puts it on C-Down.
- **What to report:** how aiming feels (the stick's speed, the inverted pitch, the slow start),
  the slingshot's and string's look in first person and Z-aim, the dry first raise's delay, the
  seed's flight, puff and sounds, the nut's flash and the stun, the letterbox, and the C-Up look.

## Milestone 5b: the pause menu

**Status:** split agreed (2026-10-07). The user chose:
- two parts, 5b-1 (the frame and the item page) and 5b-2 (the dungeon map and the game over), as
  below;
- outside dungeons, the world map page showing its background only, its contents and `INIT`'s
  world map points logged;
- logged: `INIT`'s Link portrait for the equipment page, the quest page's song states' ocarina
  calls (the states ported), the L press's debug editor (the menu stays open), B's save prompt
  (5c's);
- the menu at the C's 30 frames a second, and the scene behind it kept from the `SETUP` frame's
  lists, without the anti-aliasing filter.

### The survey

Line counts are the decomp's (`52a510f`), for this ROM's branches only: `gc-eu-mq-dbg`
(`PLATFORM_GC`, `OOT_PAL`, `OOT_MQ`, `DEBUG_FEATURES`; the language is English). "Port today" is
`16b8e94` (milestone 5a).

**The frame** (`z_kaleido_scope.c`, 4,751 lines: 27 functions and about 1,900 lines of tables):

| C | Lines | What | Port today |
|---|---|---|---|
| `KaleidoScope_Update` | 1,084 | Every pause state. `INIT` (374: the buttons saved, icons loaded and greyed by age, the language textures, the dungeon map loaded, the world map's points and the trade marker (190), the equipment page's Link portrait); `OPENING_1`/`_2` (33); `MAIN` (131: Start closes, B opens the save prompt, the item equip's state, the quest page's song states); `SAVE_PROMPT` (116); the game over's 10 to 17 (300); `CLOSING`, `RESUME_GAMEPLAY` (69) | The game over states (ADR 0032) |
| `KaleidoScope_DrawPages` | 429 | The cursor's colour cycle; the stick's repeat filter (input in the draw: first 10 frames, then every 3); the three other pages, then the active one, each a page matrix (translated 93.55 out, scaled 0.78, turned by its pitch), its 15 background tiles, then its contents; the save and game over prompt page | The prompt's part (`KaleidoScope_UpdatePrompt`) |
| `KaleidoScope_DrawUIOverlay` | 495 | The info panel, the L and R buttons, the name panel (the item's or point's name), the prompts ("(C) to Equip", "To Map", ...) | Not ported |
| `KaleidoScope_SetVertices`, `_SetPageVertices` | 657 | Every page's quads each frame: the 3x5 background grid (80x32 tiles, the per-column colour gradient), the items', equipment's, quest's, cursor's, overlay's and prompt's quads | Not ported |
| Cursor, page turns, name panel, opening (`_SetDefaultCursor`, `_SetupPageSwitch`, `_HandlePageToggles`, `_DrawCursor`, `_DrawPageSections`, `_UpdateCursorVtx`, `_UpdatePageSwitch`, `_UpdateNamePanel`, `_UpdateOpening`, `_SetView`, `_MoveCursorToSpecialPos`, the quad helpers) | about 470 | The page ring (item, map, quest, equipment), R right and Z left, the arrows' stick repeat (10 frames); a turn is 16 steps of the eye round the box of pages; the name panel's timer (70, shown 40) | Not ported |
| `_LoadDungeonMap`, `_UpdateDungeonMap`, `_OverridePalIndexCI4` | 75 | The dungeon map page's two room textures and the current room's palette index | Not ported |
| `_DrawGameOver` | 44 | "GAME OVER" with its scrolling mask (two textures, two cycles) | Not drawn |
| `_SetupPlayerPreRender`, `_ProcessPlayerPreRender` | 30 | The equipment page's Link, rendered to a 64x112 image | Not ported |
| `_GrayOutTextureRGBA32` | 20 | The wrong age's icons greyed when the menu opens (`(r + 2g + b) / 7`) | Not ported |

The rest of the frame:
- **`z_kaleido_setup.c`** (`KaleidoSetup_Update`, 66; `KaleidoSetup_Init`, 58): Start opens the menu
  on the last page viewed (from the page to its right, scrolling left); the setup sets
  `R_UPDATE_RATE` 2, the letterbox to 0, and calls `func_800F64E0(1)`. The port has the update's
  conditions in part and logs.
- **`z_kaleido_scope_call.c`** (`KaleidoScopeCall_Update`, 66; `_Draw`, 13): the letterbox's wait,
  the background's prerender, then `KaleidoScope_Update`; the draw from `READY` on. Ported for the
  game over.
- **`z_kaleido_prompt.c`** (36): ported (the game over).
- **`z_kaleido_manager.c`** (113): the overlays share one RAM area with `ovl_player_actor`. A port
  without overlays needs none of it, nor the `Object_ReloadAll` and `func_800418D0` that make up
  for the menu writing over object memory.
- **The HUD's part** (`z_parameter.c`): the START button (`startAlpha`, 2860), the icon flying to
  the C button (3450-3500, drawn by the HUD in its overlay, not by the menu), the B button's
  "SAVE" label, the HUD hidden for the debug editor (3224).

**How Play pauses** (`IS_PAUSED`: `state != PAUSE_STATE_OFF` or `debugState` open):
- **Stops** (`z_play.c`): `gameplayFrames`, the room requests, `CollisionCheck_*` (1024-1033),
  `Actor_UpdateAll` with Player (1038), the cutscenes (1042, 1045), `Effect_UpdateAll` and
  `EffectSs_UpdateAll` (1048, 1051), `Message_Update` (1092-1101: `KaleidoScopeCall_Update`
  instead), every camera (1130-1147), `Environment_Update`'s body (`z_kankyo.c` 945: time, rain,
  the time-based music, the lights' blend), `Map_Update` (`z_map_exp.c` 568), the buttons'
  status (`func_80083108`, `z_parameter.c` 4055).
- **Keeps running:** the object loads, `Skybox_Update`, `Interface_Update` (the health and rupee
  meters, the HUD's alphas), `SfxSource_UpdateAll`, `Letterbox_Update`, the fade.
- **Sound:** `func_800F64E0(1)` on the Start frame (`z_kaleido_setup.c` 130): `NA_SE_SY_WIN_OPEN`
  and the global mute, which each sequence takes by its mute behaviour (most music stops starting
  notes and halves the held ones; Hyrule Field's and the ambience keep on softer; the world's
  sound effects go quiet, the system's and the ocarina's channels don't). `func_800F64E0(0)` when
  Start closes (`NA_SE_SY_WIN_CLOSE`, the unmute), on that frame, not when the pages have closed.
  The port has `func_800F64E0` and the mute behaviours.
- **The rate:** `R_UPDATE_RATE` 2 while paused (30 frames a second), back to 3 at
  `RESUME_GAMEPLAY` (4707). The port runs a fixed 20 Hz (`eng_math::GAME_HZ`), and the audio's
  offline renderer takes 3 retraces a frame.
- **Port today** (`play.rs` 774-925): actors, collision, cutscenes, effects and the cameras stop;
  `Message_Update` gives way to the game over's update. The environment isn't stopped (the C stops
  it), `kaleido_setup_update` doesn't check `IS_PAUSED`, the cutscene index, the shooting
  gallery, the magic or the bowling alley, and sets no state.

**The scene behind the menu** (`z_play.c` 1271-1426, `PreRender.c`):
- `SETUP`: the scene is drawn as usual (no HUD), copied to `gZBuffer` with its coverage, and that
  frame isn't shown. `PROCESS`: the next frame runs `PreRender_ApplyFilters` on the CPU: the VI's
  edge anti-aliasing redone in software on every partly covered pixel (`AntiAliasFilter`, 136:
  each edge pixel blended towards its fully covered neighbours' second-highest and second-lowest
  values by its uncovered eighths). The divot filter runs only with a debug register
  (`R_HREG_MODE`). `READY`, for as long as the menu is up: the saved image is copied back and
  **the scene isn't drawn** (skybox, rooms, actors, effects skipped); the menu then draws on it
  without depth, and the HUD over the menu.
- The port has no coverage buffer: it draws with MSAA, so its edges are already smoothed. It has
  no framebuffer copy (one pass into one target, read back only to the CPU).

**The item page** (`z_kaleido_item.c`, 869):

| C | Lines | What |
|---|---|---|
| `KaleidoScope_DrawItemSelect` | 411 | The cursor over the 6x4 grid (left and right step to the next item in the row, then the next row; past the row's start onto the page arrow; up and down within the column, no wrap; from an arrow, column by column), C-Left, C-Down or C-Right starts the equip (the wrong age or a sold-out item: `NA_SE_SY_ERROR`), the magic arrows' start, the outlines on the three equipped slots, the 24 icons, the cursor's slot enlarged, the ammo |
| `KaleidoScope_DrawAmmoCount` | 53 | Two 8x8 digits: grey for the wrong age, grey 130 at 0, green at capacity |
| `KaleidoScope_UpdateItemEquip` | 326 | The icon's flight to the button (10 frames; its size `WREG(90)` 320 to 240 the first time, 280 after, a C quirk of the registers' reset), the magic arrows' four stages, then the C buttons' swap (C-Down and C-Right don't copy the slot for the bow's case: kept) |
| `KaleidoScope_SetCursorPos`, `_SetItemCursorPos` | 8 | The cursor's corner (the second is never called) |

Tables: `gSlotAgeReqs`, `gItemAgeReqs`, `gEquipAgeReqs` (`z_kaleido_scope.c` 751-892; the port has
`EQUIP_AGE_REQS` only), `gAmmoItems`, `sAmmoVtxOffset`, `sCButtonPosX/Y`. No `Rand` anywhere in the
menu.

**The dungeon map page** (`z_kaleido_map.c` `KaleidoScope_DrawDungeonMap`, 341; `z_lmap_mark.c`,
183; `z_lmap_mark_data_mq.c`, 531 of data):
- The page runs in the 10 dungeons and 8 boss rooms (`sInDungeonScene`); everywhere else the map
  page is the **world map** (`KaleidoScope_DrawWorldMap`, 574, and `INIT`'s points, 190).
- It draws the dungeon's title, the boss key, compass and map icons owned, the floor buttons (the
  visited floors, or all with the map; the viewed one in blue), Link's head at his floor, the boss
  skull (with the compass), the Gold Skulltula icon, the current room's palette pulsing (20-frame
  stages), and the floor's two 48x85 room maps: **CI4 textures drawn with a palette built at run
  time** (`Map_SetFloorPalettesData`, ported; `mapPalette`).
- The cursor: the floors' column and the items' column; up and down a floor reloads the maps.
- `PauseMapMark_Draw` (with the compass): the viewed floor's chests (hidden once opened,
  `Flags_GetTreasure`) and the boss's mark. The Master Quest Deku Tree's: 3F chests 2 and 6, 2F 1,
  1F 3, B1 0, 4 and 5, B2 the boss. The boss mark's pulse runs only in the boss scenes, where the
  marks aren't drawn: it stays 1.0.
- **The pack has `gMapDataTable` and the minimap's `gMapMarkDataTable` (ADR 0040), not the pause
  map's `gPauseMapMarkDataTable`** (its own struct, `PauseMapMarkData`): the importer needs to read
  it.

**The equipment and quest pages** (`z_kaleido_equipment.c`, 715; `z_kaleido_collect.c`, 873):
their backgrounds are `KaleidoScope_DrawPages`'s (the frame); the files draw only their contents,
and their cursors live in those draws. Logged, the cursor stays on the arrow it came in by, and
the page turns on. Logging `KaleidoScope_DrawEquipment` also leaves out its A-button equipping,
which Start's stand-in keeps doing. The quest page's song states in `KaleidoScope_Update` are
reachable only from its contents (the ocarina isn't ported).

**The game over** (ADR 0032's states are ported): drawn, it's `KaleidoScope_DrawGameOver` and
`DrawPages`' prompt page with `icon_item_gameover_static`'s message and prompts.

**The textures** (all English; the PAL XMLs for four of them):

| File | What the menu uses | In the pack |
|---|---|---|
| `icon_item_static` | 90 item icons (RGBA32 32x32), 69 language-neutral page tiles (IA8 80x32), the cursor's corners, the info panel, L and R, A, B, C symbols, the prompt cursor | The HUD's item icons (rectangles, another setup) |
| `icon_item_24_static` | 20 icons 24x24 (dungeon items, quest items) | Two, for text |
| `icon_item_nes_static` | 9 English page tiles (the titles), 10 dungeon titles, the prompts' labels, "Yes"/"No", the save prompt | No |
| `item_name_static` | 123 item names (IA4 128x16) | No |
| `map_name_static` | The world map's 12 point names and 22 area names | No |
| `icon_item_dungeon_static` | 17 floor buttons, Link's head, the skull | No |
| `map_48x85_static` | 68 room maps (CI4 48x85) | No |
| `icon_item_field_static` | The world map (CI8 216x128 in pieces, clouds, area boxes) | No |
| `icon_item_gameover_static` | "GAME OVER" (three parts and the mask), "Continue?" | No |
| `parameter_static` | The ammo digits, the equipped outline, the map's chest and boss marks | The digits |

`map_i_static` is the HUD's minimap, not the menu's.

**Drawing it in the port:**
- **No runtime display lists** (ADR 0006): each texture with its setup is a sprite bake (ADR
  0017), drawn under a transform with dynamic colours. The menu's quads under the page matrices
  fit that: a quad of the C's own vertices (`Quad::Vtx`), the vertex colours per draw
  (`DrawParams::vertex_colors`, the gradient and the alpha). About 450 bakes for the item page and
  the frame (icons and their greyed copies, names, tiles, the overlay), about 100 more for the map
  page and the game over.
- **The pages need a projection of their own:** `View_LookAt` from `eye` (0, 0, 64 at rest; the
  page turns move it) to the origin, fovy 60, near 10, far 12800, the 320x240 viewport. The pages
  stand 93.55 out, so the page behind the eye and the side pages cross it. The HUD's way (the A
  button: a projective transform divided on the CPU) doesn't clip, so they'd mirror. They need
  the GPU's clipping: a list drawn with its own view and projection, in the overlay's 4:3 frame,
  without depth, back faces culled, after the scene and before the HUD.
- **The dungeon map's room textures** need their palette at draw time (16 colours, one index
  pulsing): the renderer has only decoded RGBA textures. A colour-indexed texture with a palette
  per draw is an engine feature (and a pack format change).
- **The greyed icons** are bytes the importer must make (the runtime has no ROM).
- **Link's portrait** (render to texture) only if the equipment page's contents come in; not
  needed here.

**Start's stand-in and what relies on it:**
- **Equipment half** (`SaveContext::equip_owned_unworn`): `Task::Equip` in `mido_shop` and the
  routes that extend it (`MidoShop`, `NewSaveDekuTree`, `NewFileDekuTree`); `--test chest`,
  `--test shop`, `--test playthrough`, `--test sfx_route`. The goldens `mido_shop`,
  `mido_shop_audio`, `new_save_deku_tree` and `new_file_deku_tree` press Start: with a real menu
  opening and closing, their later frames shift.
- **Item half** (`equip_sticks_on_empty_c_left`, `equip_nuts_and_slingshot_on_empty_c`):
  `--test stick` (Start puts sticks on C-Left), `--test slingshot` (Start's nuts and slingshot;
  room 10's chest then Start). The presets call `equip_item_on_c` directly and stay.
- `PlayExt::equip_owned_unworn` runs both halves despite its name.

### The split

About 4,800 lines of C ported, with tables, plus two engine features: three times 5a. In two:

- **5b-1, the frame and the item page** (about 3,700):
  - `KaleidoSetup` whole, `KaleidoScopeCall`, `KaleidoScope_Update`'s states but the game over's
    (ported) and what the questions leave out; `_DrawPages`, `_DrawUIOverlay`, `_SetVertices`,
    `_SetPageVertices`, the cursor, page turns, name panel and opening whole; the four pages'
    backgrounds; the HUD's START button, flying icon and "SAVE" label.
  - The item page whole (`z_kaleido_item.c`), with the age tables and the greyed icons.
  - Play paused as the C pauses it (the environment too), `R_UPDATE_RATE` 2 while paused, the
    scene behind (below).
  - The engine: the pause list with its own view and projection.
  - Start's stand-in: only its equipment half, run when the menu closes (`RESUME_GAMEPLAY`, where
    `Player_SetEquipmentData` runs).
  - The save prompt (B) logs (5c's).
  - **Exit:** from a debug start with the slingshot owned and C-Right empty, Start, the cursor to
    the slingshot, C-Right, Start: the menu closes and the slingshot is on C-Right. The golden
    `pause`, with screenshots of the item page.
- **5b-2, the dungeon map and the game over** (about 1,100):
  - `KaleidoScope_DrawDungeonMap` whole, `_LoadDungeonMap`, `_UpdateDungeonMap`,
    `_OverridePalIndexCI4`, `z_lmap_mark.c` whole, `gPauseMapMarkDataTable` imported.
  - The engine: colour-indexed textures with a palette per draw.
  - The game over screens drawn (`_DrawGameOver`, the prompt page).
  - **Exit:** the map page from a debug start (the visited floors, the chests' marks with the
    compass, a floor changed with the stick); the game over's screens in its golden.

**The scene behind the menu** (both parts): the C saves the frame and stops drawing the scene.
The port would do the same without a copy: at `SETUP` it keeps that frame's scene lists and
redraws them unchanged while `READY` (no actor's draw, no draw-time state), the menu over them.
`PreRender_ApplyFilters` is left out: it redoes the console's edge anti-aliasing from its
coverage values, which the port doesn't have (MSAA smooths the same edges); the divot filter is
debug-only.

## Milestone 5b-1: the pause menu's frame and the item page

**Answer:** done. Enter (Start) opens the pause menu as the game does: the scene stops behind it,
the pages turn up into view at 30 frames a second, and the item page's cursor, its name panel and
"(C) to Equip" work; C-Left, C-Down or C-Right sends the item's icon flying to the button. R and Z
turn the four pages (the equipment, quest and map pages show their backgrounds), Start closes the
menu and the game resumes, wearing what's owned (the equipment page's stand-in). The exit holds;
its run is the golden `pause`, with `pause_item` and `pause_map` its screenshots.

The pack is format 24, in `out/data21`. Decisions are in
[ADR 0047](adr/0047-the-pause-menu.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 68 to 70):
- `test-pause.bat`: the milestone's tests;
- `game-pause.bat`: the game inside the Deku Tree with the slingshot owned but on no button
  (`deku-tree-slingshot-owned`): Enter opens the menu;
- `sandbox-pause.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** 5b's split as agreed (2026-10-07, above); 5b-2 (the dungeon map page and
the game over drawn) followed, below.

### What was built

**The menu** (`oot_game::kaleido`, ADR 0047):
- `KaleidoSetup_Init` (the `PauseContext`'s defaults) and `KaleidoSetup_Update` whole (the
  shooting gallery, magic and bowling alley checks have nothing ported to read): Start opens on
  the last page viewed, `R_UPDATE_RATE` 2, the letterbox closing, `func_800F64E0(1)`.
- `KaleidoScopeCall_Update` and `_Draw`: the letterbox's wait, the background's prerender, the
  overlay loaded (its statics fresh, `KaleidoStatics`), the menu's update and draw.
- `KaleidoScope_Update`'s states but the save prompt's (logged, 5c): `INIT` (the world map's points
  and the trade marker logged), `OPENING_1`, `OPENING_2`, `MAIN` (Start closes; B logs the save
  prompt; the equip; the quest page's song states with their ocarina calls logged), `CLOSING`,
  `RESUME_GAMEPLAY`; the game over's states as ADR 0032 had them.
- The frame whole: `KaleidoScope_Draw`, `_DrawPages` (the cursor's colour cycle, the stick's
  repeat, the four pages' matrices and backgrounds), `_DrawPageSections`, `_DrawUIOverlay` (the
  info panel, L and R pulsing, the cursor on the arrows, the name panel, the prompts),
  `_SetVertices`, `_SetPageVertices`, `_DrawCursor`, `_UpdateCursorVtx`, `_SetupPageSwitch`,
  `_HandlePageToggles` (L logs the debug editor), `_UpdatePageSwitch`, `_UpdateNamePanel`,
  `_UpdateOpening`, `_SetDefaultCursor`, `_MoveCursorToSpecialPos`, `_SetView`,
  `_GrayOutTextureRGBA32`, `KaleidoScope_UpdatePrompt`; the REGs (`PauseRegs`), the tables
  (`gSlotAgeReqs`, `gItemAgeReqs`, `gPageSwitchNextButtonStatus`, the pages' colours and quads).
- The item page whole (`z_kaleido_item.c`): `KaleidoScope_DrawItemSelect`, `_DrawAmmoCount`,
  `_UpdateItemEquip`, `_SetCursorPos`, `gAmmoItems`.
- Logged: `KaleidoScope_DrawEquipment`, `_DrawQuestStatus`, `_DrawDungeonMap` and
  `PauseMapMark_Draw` (5b-2), `_DrawWorldMap`, `_DrawGameOver` (5b-2), the Link portrait.

**The draw** (ADR 0047): a recorder of the C's quads (`KaleidoGfx`), each a sprite bake placed by
its vertices; about 370 bakes, the wrong age's icons greyed by the importer
(`BakeSegment::GrayRgba32`); the cursor's vertices resolved when the frame is drawn
(`KaleidoScope_UpdateCursorVtx` at the end of the draw, as the next update's race makes it).

**The engine:** the pause list (`DrawLists::pause`, `pause_view`): drawn after the 3D lists,
before the overlay, in its own perspective, the GPU clipping it. `R_UPDATE_RATE` as play state
(`PlayState::r_update_rate`): the app's loop, the letterbox, the flash's fade and the offline
audio's retraces follow it.

**Play paused** as the C pauses it: `Environment_Update`'s body stops too; from the background's
`PROCESS` on, Play's draw-time state stops and the scene's lists are redrawn as saved, with the
saved frame's fills; the saving frame skips the overlay elements; `KaleidoScopeCall_Draw` before
`Interface_Draw`.

**The HUD** (`z_parameter.c`): the START button and its label, the B button's label ("SAVE"),
`Interface_SetDoAction`'s paused branch, the icon flying to its C button, `func_80084BF4`'s
opening branch.

**Start's stand-in:** down to its equipment half, run as the menu resumes; its item half and
`PlayState::pause_menu_equip` gone. The preset `deku-tree-slingshot-owned` (the slingshot on no
button). The scripted runs' `Task::Equip` opens and closes the menu.

**The exit run:** `Route::Pause` (`--script pause`): from the Deku Tree's spawn, the menu opened,
the cursor to the slingshot, C-Right, R to the map page, the menu closed. The sandbox's trace
carries the menu's state while paused.

### Results

**Tests.** `cargo test --release --workspace` (`target/game22`, `OOT_DATA_DIR=out/data21`):
624 passed, 0 failed, 1 ignored (603 before). 21 are new, with their expectations from the C:
- **`oot_actors --test pause`** (13): Start opening the menu frame by frame (the right page and
  its eye, `R_UPDATE_RATE` 2, `NA_SE_SY_WIN_OPEN`, the prerender's two frames, `INIT`, 8 frames of
  `OPENING_1` with the alpha 15 a frame, 8 of `OPENING_2`, then the item page, the eye at (0, 0,
  64), the panel, START and L and R in place, the page's buttons, "SAVE" on B, "DECIDE" on A, the
  first draw's cursor pushed from the sticks to the nuts); the cursor's moves (along the row,
  never back to column 0, the arrows, column by column from an arrow, up and down without
  wrapping); the held stick's moves on frames 0, 11 and 14; C-Right's equip (the icon from (-940,
  240) by (208, 86) a frame, `WREG(90)` 320 to 240, the swap after it, the next equip shrinking 4
  a frame); a wrong age's error and its grey icon; R's 16-frame turn (the eye by (-4, -4), L and R
  out and back, `NA_SE_SY_WIN_SCROLL_RIGHT`, the map's buttons); Start closing (8 frames,
  `NA_SE_SY_WIN_CLOSE`, the resume: the buttons as they were, 20 frames a second); the equipment
  stand-in at the resume; B's save prompt logged; Start refused during the fade-in; every quad of
  a session baked and covering its texture; a bake's vertex order; the scene behind stopped.
- **`--test pause_run`** (1): the exit run.
- **`oot_game` `kaleido::tests`** (7): the greying, the age tables, the page turns' eye steps and
  pages, the C buttons' swap, a magic arrow onto the bow, the bow beside a bow with arrows (the
  slot only C-Left copies), the bakes.
- Rewritten for the menu: `--test stick` (the menu puts sticks on C-Left), `--test slingshot`
  (nuts and the slingshot through the menu, the swap; room 10's chest then the menu), `--test
  shop` (Start opens the menu, not while talking; its resume wears the shield and the sword).

**The exit run** (`Route::Pause`, `--script pause`, the `deku-tree-slingshot-owned` preset):
`menu_opened` 52, `cursor_on_item` 55, `item_equipped` 66, `page_turned` 82, `menu_closed` 93.

**The goldens.** Against milestone 5a's build (`target/game21`, 16b8e94, on data20: 92/92
identical, its outputs in `out/golden_base5b`): the four runs that wore their equipment with Start
(`mido_shop`, `mido_shop_audio`, `new_save_deku_tree`, `new_file_deku_tree`) now open and close
the menu (32 frames paused each time, 66 frames longer in all, with the menu's sounds), proven by
putting the old instant equip back (the baseline's bytes); every other case the same bytes. New:
`pause` (the trace and the game resumed), `pause_item` (cut at frame 60: the icon in flight) and
`pause_map` (cut at 82), each the same bytes over two runs. Logged in
[golden/README.md](../golden/README.md): 96 hashes, 71 cases.

### Decisions

- **[ADR 0047](adr/0047-the-pause-menu.md):** the recorder of the C's quads and its bakes; the pause
  list in its own perspective; the cursor's race kept as its result; `R_UPDATE_RATE` as play
  state; the scene behind kept by stopping Play's draw-time state, without the anti-alias filter;
  the overlay's statics; the greyed icons from the importer; what logs; the stand-in's equipment
  half at the resume; the HUD's part; the faithful bugs; pack format 24.
- **The first open's cursor** lands on the second item, as the C's does: `KaleidoSetup_Init`
  (every `Play_Init`) leaves `cursorItem[PAUSE_ITEM]` at `PAUSE_ITEM_NONE`, and the first idle draw
  pushes the stick right (`stickAdjX` 40).

### Known gaps

- **5b-2:** the dungeon map page's contents (its CI4 room maps want a palette per draw) and its
  marks (`gPauseMapMarkDataTable`, not in the pack yet); the game over's message and prompt page.
- **Logged:** the equipment and quest pages' contents (with the equipment page's A equipping,
  whose stand-in runs at the resume), the world map's, the Link portrait, the debug inventory
  editor (L), the save prompt (B, 5c).
- **The frame the background is saved on is shown** with the HUD; the console doesn't show it.
- **A wide window shows the side pages** at its edges.
- **Player's overlay statics** aren't reset when the menu closes (the C reloads `ovl_player_actor`).
- **`gSaveContext.worldMapArea`** isn't read from the scene (only the world map uses it).

### Fixes found while building

- None in the old code; the C's cursor race was found in the reading (ADR 0047).

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-pause.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-pause.bat
```

### By hand

`game-pause.bat` (or menu 69). Enter is Start, WASD the stick, J, K, L the C buttons (C-Left,
C-Down, C-Right), R is R, Q is Z, T is L, E is B, Space A.
1. **Opening:** stand still a moment after the scene fades in, then Enter: the sound of the menu
   opening, the music quietens, the pages swing up from below into a box around the view with the
   item page in front ("SELECT ITEM"), the HUD's START button shows "Return", B "Save", A
   "Decide". The cursor starts on the Deku nuts (the second item), as on the console.
2. **The cursor:** WASD moves it. Left and right step to the next item in the row, then on to the
   next rows; past the edge it lands on the L or R arrow (a chime), and pushing on turns the page
   after a moment. Up and down stay in the column. Hold a direction: one step, then a pause, then
   quick repeats. The item's name shows in the panel, alternating with "(C) to Equip".
3. **Equipping:** on the slingshot, L (C-Right): its icon flies to the C-Right button, shrinking.
   Equip it on J (C-Left) too: it moves, and the sticks go to C-Right.
4. **Pages:** R and Q (Z) turn the box to the next page right or left (the map, quest and
   equipment pages show their backgrounds; their contents come later). The C buttons dim off the
   item page.
5. **Closing:** Enter: the pages swing down, the music comes back, the game goes on; the slingshot
   is on C-Right and works.
- **What to report:** the opening and closing's speed and swing, the page turns, the cursor's feel
  (the repeat's delay and speed), the icon's flight, the panel's name and prompt, the HUD's look in
  the menu, and anything drawn wrong (a texture, a colour, a page edge).

## Milestone 5b-2: the dungeon map page and the game over drawn

**Answer:** done. In a dungeon the pause menu's map page is the C's: the dungeon's title, the
boss key, compass and map owned, the visited floors' buttons (every floor's with the map), Link's
head at his floor, the boss's skull with the compass, the Gold Skulltula icon, and the viewed
floor's two room maps with the visited rooms in colour and the room Link is in pulsing on his
floor; the compass marks the floor's unopened chests. The cursor moves over the floors (each
loads its maps) and the dungeon items. The game over draws "GAME OVER" fading in from orange to
dark red over the scene, then the window turning in with "Would you like to save?" and
"Continue playing?", their cursor and "Yes", "No". Both exits hold; their runs are the goldens
`dungeon_map` (with `dungeon_map_1f`, `dungeon_map_2f`) and `game_over` (with
`game_over_message`, `game_over_save`, `game_over_continue`).

The pack is format 25, in `out/data22`. Decisions are in
[ADR 0048](adr/0048-the-pause-map-and-the-game-over.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 71 to 75):
- `test-dungeon-map.bat`: the milestone's tests (then 5b-1's pause tests);
- `game-dungeon-map.bat`: the game inside the Deku Tree with the compass and 3F to 1F visited
  (`deku-tree-compass`): Enter, then R, opens the map page;
- `sandbox-dungeon-map.bat`: the map exit's run headless, its trace and screenshots;
- `game-game-over.bat`: the game on the top floor by a Deku Baba with a quarter heart
  (`deku-tree-quarter-heart`): its bite, then the game over;
- `sandbox-game-over.bat`: the game over's run headless, its trace and screenshots.

**Status of the plan:** 5b is done (5b-1 and 5b-2, as split on 2026-10-07); 5c (saving) is next.

### What was built

**The palette design** (the user chose a per-draw image of three, 2026-10-09; ADR 0048):
- the room maps are drawn from the game's own texels: `interfaceCtx->mapSegment` is bytes
  (`MapState::segment`), `KaleidoScope_LoadDungeonMap` copies `map_48x85_static`'s two maps in
  from the pack, `KaleidoScope_OverridePalIndexCI4` runs on them as written, and the recorder
  decodes the CI4 texels through the palette `gDPLoadTLUT_pal16` loaded
  (`KaleidoGfx::quad_ci4`, `eng_gbi`'s decoder, the importer's);
- **the engine:** `eng_gfx::DrawParams::image` (`DrawImage`, RGBA8) replaces a draw's texture
  slot 0; the renderer gives the mesh's instance its own texture and bind groups (the mesh's
  samplers kept), uploading when the texels change.

**The dungeon map page** (`oot_game::kaleido::map`, `z_kaleido_map.c`):
`KaleidoScope_DrawDungeonMap` whole: the cursor over the floors' column (visited floors, or all
with the map) and the items' column, the arrows, the floor's button under the cursor enlarged,
the title, the items, the buttons, Link's head (`VREG(30)`), the skull (`sSkullFloorIconY`), the
Gold Skulltula icon (`gAreaGsFlags`), the current room's pulse (`mapBgPulse*`, palette entry 14),
the room maps point sampled.

**The loads** (`z_kaleido_scope.c`): `KaleidoScope_LoadDungeonMap`, `_UpdateDungeonMap`
(`Map_SetFloorPalettesData`, the current room on index 14 on Link's floor),
`_OverridePalIndexCI4`; `INIT`'s call.

**The marks** (`oot_game::kaleido::lmap_mark`, `z_lmap_mark.c`): `PauseMapMark_Init`, `_Clear`,
`_DrawForDungeon` (the viewed floor's chests until opened, the boss's mark with its pulse in the
boss scenes, `GREG(92)`, `GREG(93)`), `_Draw`; `gBossMarkState`, `gBossMarkScale` on the
`PauseContext`.

**The game over** (`z_kaleido_scope.c`): `KaleidoScope_DrawGameOver` (three screen-space
rectangles, `KRect`, at the end of the pause list, the mask on tile 1 scrolling by `VREG(89)`);
`KaleidoScope_DrawPages`' prompt page in a game over (its 15 tiles, `sGameOverTexs`, the message,
the cursor, "Yes", "No"); `INIT`'s `VREG(88)` and `SHOW_WINDOW`'s pitches, panel, START alpha,
message, buttons, `XREG(5)` and alpha, which ADR 0032 had left to the draw.

**The bakes:** 61 more (the titles, floor buttons, head, skull, dungeon items, Gold
Skulltula, room map, marks under both combiners, the prompt page's tiles and labels, the prompt
cursor); `sprite::Tile1`, a sprite bake's second texture on tile 1 with a dynamic tile size, for
"GAME OVER"'s three parts. The pause list draws screen-space commands (`DrawParams::screen`).

**The pack** (format 25): `table/map` gains `gPauseMapMarkDataTable` (from
`z_lmap_mark_data_mq.c`, each entry with its `Vtx` array) and `map_48x85_static`'s bytes.

**The debug starts and runs:** the presets `deku-tree-compass` and `deku-tree-quarter-heart`;
`Route::DungeonMap` (`--script dungeon-map`: Start, R, the stick right onto the floors and up,
Start) and `Route::GameOver` (`--script game-over`: the Deku Baba's bite at `DEKU_BABA_START`, No
at the save prompt, Yes at the continue prompt, the respawn).

**Still logged:** the world map's contents, the equipment and quest pages' contents, the save
prompt's page (5c's).

### Results

**Tests.** `cargo test --release --workspace` (`target/game23`, `OOT_DATA_DIR=out/data22`):
683 passed, 0 failed, 1 ignored; 15 are new, with their expectations from the C:
- **`oot_actors --test dungeon_map`** (8): the menu's `INIT` loading maps 4 and 5 (1F) with room
  0's palette index 10 moved to 14 and 1F's palette; the pulse's steps, its two colours, and the
  room maps drawn texel by texel through the palette; the floors' column (2F: maps 2 and 3, rooms
  0 to 2's palettes, no recolouring; 3F; the unvisited floors skipped, B1 and B2 below), the
  buttons' colours and the enlarged one; the items' column and the arrows; the title, the
  compass, Link's head (-21), the skull (-47), the Gold Skulltula icon only with all five; the
  marks on 1F, 2F and 3F at their points, hidden once opened, under the cursor's combiner or
  `G_CC_MODULATEIA_PRIM`; every quad baked and covering its texture; the exit's run.
- **`--test game_over_screens`** (4): "GAME OVER"'s rectangles from (64, 98), the mask's scroll
  and the colours' 30 steps; the window's turn (the pitches, the panel, the message rising 3 a
  frame, the buttons, the alpha) and its end values; the save prompt's tiles, message, cursor and
  choices where the C puts them, No, then "Continue playing?"; every quad and rectangle baked;
  the exit's run.
- **`oot_import --test pack`** (1): `gPauseMapMarkDataTable` found in `ovl_kaleido_scope` with
  its `Vtx` pointers, and `map_48x85_static` as the ROM has it (68 maps).
- **`oot_game` `kaleido::tests`** (2): `KaleidoScope_OverridePalIndexCI4`, the game over's bakes
  (the dynamic prim with its LOD fraction 80, tile 1's size).
- Changed: `the_menus_bakes_are_unique_and_cover_the_pages` (75 page tiles: the game over's 15).

**The exit runs:**
- `Route::DungeonMap` (`deku-tree-compass`): `menu_opened` 52, `page_turned` 68,
  `floor_changed` 71, `menu_closed` 82.
- `Route::GameOver` (`deku-tree-quarter-heart`): `died` 31, `save_prompt` 190,
  `continue_prompt` 192, `respawned` 269.

**The goldens.** Against 5b-1's build (`target/game22`, on data21: 96/96 identical, its outputs in
`out/golden_base5b2`): `pause_item` and `pause_map` differ only where the map page now draws its
contents (the title, 1F, Link's head, room 0), proven by not drawing them (the baseline's
bytes); every other case the same bytes. New: `dungeon_map`, `dungeon_map_1f` (frame 70),
`dungeon_map_2f` (72), `game_over`, `game_over_message` (170), `game_over_save` (191),
`game_over_continue` (193), each the same bytes over two runs. Logged in
[golden/README.md](../golden/README.md): 105 hashes, 78 cases.

### Decisions

- **[ADR 0048](adr/0048-the-pause-map-and-the-game-over.md):** the room maps from the game's own
  texels on a per-draw image; the marks' table and `map_48x85_static` in the pack (format 25);
  the late-read vertices; the marks under whatever combiner is set; "GAME OVER" as screen-space
  rectangles with a two-texture sprite bake; the game over's states' draw fields; the faithful
  bugs; the debug starts.
- **The map run's floor change** pushes the stick right first: R's turn leaves the cursor on the
  map page's left arrow (`KaleidoScope_SetupPageSwitch`), as on the console.

### Known gaps

- **Logged:** the world map's contents (outside dungeons), the equipment and quest pages'
  contents, the Link portrait, the debug inventory editor (L), the save prompt and its page (B,
  5c). "No" at "Continue playing?" still respawns (the title screen isn't ported, ADR 0032).
- **The room maps' texels and palette** are read when the menu's frame is recorded; the RDP reads
  them a frame later. Nothing writes them in between.
- **The boss mark's pulse** runs only in the boss rooms, which have no marks to draw: untested
  until a boss room is reached (milestone 6).

### Fixes found while building

- **The marks didn't draw on the page looked at:** `KaleidoScope_DrawCursor` leaves its combiner
  set and `PauseMapMark_Draw` sets none, so the marks there are under the cursor's combiner,
  which wasn't baked for them; both combiners are baked now (and the bake-coverage test covers
  the map page).
- **"GAME OVER" came out plain white:** the bake's cycle type was written unshifted (1, not
  `G_CYC_2CYCLE`), so it baked one-cycle without its colours.
- **The mask's texture didn't resolve:** segment 0x0C is the importer's culling list; it's on
  0x0F.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-dungeon-map.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-dungeon-map.bat
scripts\run\sandbox-game-over.bat
```

### By hand

Enter is Start, WASD the stick, R is R, Space A.

`game-dungeon-map.bat` (or menu 72):
1. **The map page:** Enter, then R. The page shows "Inside the Deku Tree", the compass, the 3F, 2F
   and 1F buttons (1F in blue), Link's head beside 1F, a skull lower down (the boss's floor), and
   1F's map: the central room in a colour that slowly pulses between blue and green, and a small
   chest mark on it.
2. **The floors:** the cursor arrives on the L arrow; D puts it on 1F (the button grows). W: 2F,
   its maps (the central room and the rooms beside it in blue, not pulsing), its chest mark; W
   again: 3F (two marks); W once more does nothing; S back down to 1F, and S below it does
   nothing (B1 and B2 aren't visited). Each move clicks.
3. **The items:** D from a floor goes to the compass (its name in the panel); D again to the R
   arrow; holding D there turns the page, as on the other pages.
- **What to report:** the pulse's look and speed, the maps' and marks' places and colours, the
  cursor's feel on the floors and items, anything drawn wrong.

`game-game-over.bat` (or menu 74):
1. **Dying:** stand still: the Deku Baba bites and Link falls.
2. **"GAME OVER":** fades in over the scene, dark red letters with a flickering orange edge.
3. **The window:** turns in with "Would you like to save?", a green glow on Yes; D moves it to No,
   Space takes it: "Continue playing?"; Space on Yes: the screen goes black and Link starts again
   at the entrance with three hearts. (Yes at the save prompt "saves" without writing anything:
   5c.)
- **What to report:** the message's fade and flicker, the window's turn and speed, the prompts'
  look, the cursor's glow.

## Milestone 5c: saving

**Answer:** done. B in the pause menu turns the save prompt in, "Would you like to save?" with its
cursor on Yes; Yes saves (the chime, the scene's flags, `Sram_WriteSave` to the file and its
backup) and the menu closes, as on GameCube; No, B or Start close it unsaved. The game over's Yes
saves too. The save is the cartridge's SRAM in the C's bytes: `out\saves\<ROM SHA-1>.sra` for
`--file N`, an image in memory for the debug starts and the sandbox. `--file N` loads a file as the
file select would (`Sram_OpenSave`'s entrance, three hearts at least), `--file N --new-file` makes
one, F5 is the console's reset, and `ootx sram` lists, verifies, erases and copies. The exit holds;
its run is the golden `save` (with its final SRAM image), with `save_prompt_turn` and
`save_prompt`.

The pack is format 26, in `out/data23`. Decisions are in [ADR 0049](adr/0049-saving.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 76 to 79):
- `test-save.bat`: the milestone's tests (then 5b's pause and game over tests);
- `game-save.bat`: the game inside the Deku Tree with two hearts and the slingshot on no button
  (`deku-tree-save`), a debug start on an SRAM in memory: equip, save, F5;
- `sandbox-save.bat`: the exit run headless, its trace, screenshots and SRAM image;
- `game-file.bat`: a file of the save file (`game-file.bat 2`, `game-file.bat 2 --new-file`).

**Status of the plan:** milestone 5 is done (5a, 5b-1, 5b-2 and 5c); milestone 6, Gohma, is next.

**Status:** decided (2026-10-09). The user chose:
- the save file in the repo's ignored `out\saves` (`OOT_SAVE_DIR`, which `_env.bat` sets), named by
  the ROM's SHA-1 as the packs are;
- `--file N` for the file select's load, `--file N --new-file` for a new file in an empty slot, the
  debug starts on an SRAM in memory, F5 the console's reset;
- `oot_game` never touching the disk: the SRAM an image on the play state, the apps binding it to
  the file, the sandbox on a fresh image unless `--sram PATH`;
- every `Save` field in `SaveContext` with the C's names, the slot's tail from the fields the port
  has (zeros elsewhere);
- `Sram_EraseSave` and `Sram_CopySave` ported, used by `ootx sram`;
- one milestone, with `Route::Save` as proposed below.

### The survey

Line counts are the decomp's (`52a510f`), for this ROM's branches only: `gc-eu-mq-dbg`
(`PLATFORM_GC`, `OOT_PAL`, `OOT_MQ`, `DEBUG_FEATURES`; the language is English). The iQue's
`Sram_ReadWriteIQue` (23) and the `OOT_VERSION < PAL_1_0` and `PLATFORM_N64` branches aren't this
ROM. "Port today" is `31a432e` (milestone 5b-2).

**`z_sram.c`** (1,086 lines):

| C | Lines | What | Port today |
|---|---|---|---|
| `gSramSlotOffsets`, `sSramDefaultHeader`, `SLOT_SIZE`, `CHECKSUM_SIZE` | 30 | Six slots (three files, then their backups) of `sizeof(SaveContext) + 0x28` (0x1450) bytes from 0x20; the 16-byte header (sound, Z-targeting, language, then the magic `98 09 10 21 "ZELDA"`) | No |
| `sNewSave*`, `Sram_InitNewSave` | 110 + 19 | A new file | `SaveContext::new` (ADR 0019), without `horseData` (Hyrule Field, -1840, 72, 5497, -0x6AD9), `totalDays`, `bgsDayCount` |
| `sDebugSave*`, `Sram_InitDebugSave` | 130 + 42 | The debug file | `SaveContext::debug`, without `horseData`; it takes the entrance where the C sets `ENTR_HYRULE_FIELD_0` |
| `sDungeonEntrances`, `Sram_OpenSave` | 18 + 163 | A slot from the read buffer into `gSaveContext` (`sizeof(Save)` only), then: the entrance from `savedSceneId` (a dungeon's or boss room's entrance, Ganon's tower's for the collapse; elsewhere Link's house for a child and the Temple of Time for an adult), health raised to three hearts, the scarecrow songs copied out, Zelda's letter taken back without the lullaby, the Master Sword for an adult who lacks it (`>= NTSC_1_1`: equipped on B), spoiled trade items reverted, `magicLevel` 0 | No |
| `Sram_WriteSave` | 46 | The checksum (the sum of `Save`'s big-endian halfwords with the checksum field 0) into the save, then `SLOT_SIZE` bytes from `&gSaveContext` to the slot and to its backup. It sums three times and keeps the first (the other two are dead) | Logs (the game over's Yes) |
| `Sram_VerifyAndLoadAllSaves` | 183 (about 140 without the prints) | The whole SRAM read; each slot's checksum checked; a bad slot restored from its backup; a bad backup too: `entranceIndex`, `linkAge`, `cutsceneIndex`, `dayTime`, `nightFlag`, `totalDays`, `bgsDayCount` cleared, then file 1 the debug save with "ZELDAZ" (`DEBUG_FEATURES`), the others a new save, checksummed and written to the backup and the slot; then the file select's fields (deaths, names, health capacity, quest items, 64DD flag, defence, `OOT_PAL`: health) | No |
| `Sram_InitSave` | 104 | The name entry's new file: file 1 the debug save (`DEBUG_FEATURES`), the others a new save; Link's house, child, 10:00, `cutsceneIndex` 0xFFF1 (file 1: none); the name; "ZELDAZ"; the checksum; written to the slot and its backup | In `SaveContext::file_select_new` (file 2), without the write |
| `Sram_EraseSave`, `Sram_CopySave` | 18, 41 | The file select's erase (a new save over the slot and backup) and copy | No (no caller without the file select) |
| `Sram_WriteSramHeader`, `Sram_InitSram` | 3, 54 | At the title screen: the header's magic checked (rewritten with the old language kept, `PLATFORM_GC && OOT_PAL`), the sound and Z-targeting settings read, the language checked; `DEBUG_FEATURES`: D-Right on controller 3 fills the SRAM with a ramp ("SRAM destruction") | `GameAudio::boot` sets stereo, as a fresh header gives; Player assumes "Switch" targeting |
| `Sram_Alloc`, `Sram_Init` | 7 | The read buffer (`SRAM_SIZE`, 0x8000); `Sram_Init` is empty | No |

New in the port: about 620 lines of C with the tables. The SRAM itself is `z_ss_sram.c`'s PI DMA
(`SsSram_ReadWrite`, 66), which a file stands in for.

**What calls it here:**
- **The pause menu** (`z_kaleido_scope.c`): B in `PAUSE_STATE_MAIN` (three branches: idle 12, the
  song prompt 13, the cursor on a song 13: `nextPageMode` 0, `promptChoice` 0, `NA_SE_SY_DECIDE`,
  every button disabled but A, `HUD_VISIBILITY_ALL`, `PAUSE_STATE_SAVE_PROMPT`); today they log.
  `PAUSE_STATE_SAVE_PROMPT` (116): `APPEARING` (the prompt page turns in by `promptPitch`, 314/8 a
  frame to -628, L and R move out), `WAIT_CHOICE` (A on Yes: `NA_SE_SY_PIECE_OF_HEART`,
  `Play_SaveSceneFlags`, `savedSceneId`, `Sram_WriteSave`, `SAVED` with `sDelayTimer` 3 on GameCube;
  A on No, B or Start: `CLOSING`, the buttons back, `func_800F64E0(0)`), `SAVED` (B, A, Start or the
  timer: `CLOSING_AFTER_SAVED`), `CLOSING`/`CLOSING_AFTER_SAVED` (every page and the prompt turn
  away 160/8 a frame to `YREG(8)` + 160, then `RESUME_GAMEPLAY`); `RETURN_TO_MENU` is never set
  here. On GameCube saving closes the menu: there's no "Game saved." (`sSaveConfirmationTexs` is
  `!PLATFORM_GC`).
- **`KaleidoScope_DrawPages`' save prompt page** (14 lines that differ from the game over's):
  `SAVE_TEXS(language)`'s 15 tiles, then "Would you like to save?", the cursor and Yes/No while
  `savePromptState < SAVED`, nothing after. Logged today (`log_once` bit 4).
- **Already ported for it:** `KaleidoScope_UpdatePrompt` (the stick in `WAIT_CHOICE`),
  `_DrawUIOverlay`'s "(A) to Decide", `_SetVertices`' lower origin while closing,
  `KaleidoScopeCall_Update`'s state range, `Play_SaveSceneFlags`, `Interface_ChangeHudVisibilityMode`.
- **The game over** (`PAUSE_STATE_GAME_OVER_SAVE_PROMPT`'s Yes): one line, `Sram_WriteSave`; the
  rest is ported (ADR 0032).
- **Loading:** `FileSelect_LoadGame` (`z_file_choose.c`, 82): `fileNum`, `Sram_OpenSave`, then the
  resets the port already does in `SaveContext::file_select_new`. In this debug ROM file 1 goes
  to the map select (`MapSelect_LoadGame`, `z_select.c`, 25: the save kept, the entrance chosen),
  files 2 and 3 to `Play_Init`. The file select's own SRAM writes (its options' header bytes) aren't
  ported with it.

About 900 lines of C in all, with the byte layout and the plumbing new: a little under 5b-2's
1,100, and no engine feature. No split proposed.

**The save in the C's layout** (`save.h`; big-endian, as the cartridge holds it):
- `Save` is 0x1354 bytes: seven words (`entranceIndex`, `linkAge`, `cutsceneIndex`, `dayTime`,
  `nightFlag`, `totalDays`, `bgsDayCount`), then `SaveInfo` (0x1338): `playerData`, `equips`,
  `inventory`, `sceneFlags[124]`, `fw`, `gsFlags`, `highScores`, the story flags,
  `worldMapAreaData`, the scarecrow songs, `horseData`, and the checksum last (0x1352).
- `SaveContext` adds 0xD4 bytes of play state (0x1354 to 0x1428); a slot is that plus 0x28 bytes
  of whatever follows `gSaveContext` in RAM. Only `sizeof(Save)` is ever read back, and only it is
  checksummed.

What the port's `SaveContext` holds differently or lacks:

| C | Port |
|---|---|
| `entranceIndex`, `cutsceneIndex` (s32) | `u16` (`ENTR_LOAD_OPENING`'s -1 is 0xFFFF: written sign-extended) |
| `linkAge` (s32: 0 adult, 1 child) | `adult: bool` |
| `nightFlag` (s32), `bgsFlag`, the `is*Acquired` (u8) | `bool` |
| `totalDays`, `bgsDayCount`, `newf`, `n64ddFlag`, `ocarinaGameRoundNum`, `fw` (Farore's Wind), `highScores`, `worldMapAreaData`, `scarecrowLongSong*`, `scarecrowSpawnSong*`, `horseData`, `checksum`, the `unk_*` pads | Missing |
| `sceneLayer`, `gameMode` (s32), `transFadeDuration` (u8) | `usize`, `u8`, `u16` |
| The runtime part's `dogParams`, `nayrusLoveTimer`, the timers, the magic meter's state, `minigame*`, `soundSetting`, `zTargetSetting`, `chamberCutsceneNum`, `nextDayTime`, `skyboxTime`, `dogIsLost`, `worldMapArea`, `sunsSongState` | Missing (nothing ported reads them) |

**The textures** (the page and its labels):
- `SAVE_TEXS(LANGUAGE_ENG)` is the game over's 15 tiles but one: column 2's top tile is
  `gPauseSave10ENGTex` (IA8 80x32, `icon_item_nes_static` 0xD280) where the game over has
  `gPauseGameOver10Tex`. The other 14 (`gPauseSave00`..`04`, `11`..`14`, `20`..`24`) are baked
  already (the game over's page); `gPauseSave10ENGTex` isn't: **one new bake, pack format 26**
  (`out/data23`).
- Baked already: `gPauseSavePromptENGTex` (`sSavePromptMessageTexs`), "Yes" and "No", the prompt
  cursor, "(A) to Decide" and the A symbol. `sSaveConfirmationTexs` isn't drawn on GameCube.

### The proposal

The decisions, each with a recommendation (marked):

1. **Where the save file lives, and its name per ROM:**
   - *(recommended)* the per-user folder: `%LOCALAPPDATA%\oot-clone\saves\<ROM SHA-1>.sra`, named
     like the packs (`GamePack::header().source_sha1`). It doesn't depend on `OOT_DATA_DIR`, so a new
     pack folder (`data22` to `data23`) doesn't lose it; `OOT_SAVE_DIR` overrides the folder.
   - the same folder with the version's name, `gc-eu-mq-dbg.sra`: readable, but the pack would
     have to carry the name, and a patched ROM of that version would share it;
   - `out\saves\` in the repo's ignored `out` folder: visible, but one per worktree and next to
     scratch files.
   
   The file is the cartridge's 32 KiB SRAM image, big-endian, header and six slots where the C
   puts them. It's written whole after each `SRAM_WRITE` (to a temporary file, then renamed, so a
   crash can't leave half a slot).
2. **How the game and the sandbox pick a slot** (the file select's load):
   - *(recommended)* `--file N`: the title and the file select's load as the C does them
     (`Sram_InitSram`, `Sram_VerifyAndLoadAllSaves`, `FileSelect_LoadGame` with `buttonIndex` N - 1,
     `Sram_OpenSave`, `Play_Init`). Files 2 and 3 enter where `Sram_OpenSave` says; file 1 goes
     through this ROM's map select, so it takes `--entrance` (`MapSelect_LoadGame`). Saves go back to
     the slot. `--file N --new-file` makes a new file in an empty slot as the name entry does
     (`Sram_InitSave`, the opening); a slot with a file is refused. The debug starts (`--entrance`
     with or without `--preset`, `--new-file` alone) are file 2 of an SRAM in memory: their saves
     never reach the disk (logged). In the window, F5 is the console's reset: the same start
     again, the SRAM read again.
   - every start bound to the save file, as the debug ROM would have it: a debug start's save
     overwrites its slot, and `--new-file` writes file 2 at once (`Sram_InitSave` writes when the
     name is entered), even over a file there;
   - the first option, plus a small window of the slots before play (name, hearts, deaths; load,
     new, erase, copy through the ported `Sram_*`): more work, and not the C's file select.
3. **Tests and goldens independent of the user's save file:**
   - *(recommended)* `oot_game` never touches the disk: the SRAM is a 32 KiB image on the play
     state, carried across `Play_Init` with the save, and only the apps bind it to a file. Tests
     build their images in memory. The sandbox starts from a fresh image unless `--sram PATH`;
     golden cases can hash the final image as an output (`{sram}`), so the save's bytes are
     covered. A fresh image is zeros, which every slot's checksum accepts (zero sums to zero):
     three empty files, as an erased cartridge.
   - the game crate writes files itself, and the test and golden scripts point `OOT_SAVE_DIR` at a
     temporary folder (easy to forget; the tests touch the disk);
   - as the first, but the sandbox reads the user's file (never writes it): runs would then depend
     on it.
4. **The port's `SaveContext` and the slot's bytes:**
   - *(recommended)* `SaveContext` gains every `Save` field it lacks, with the C's names, and the
     writer lays the save out field by field; the slot's tail (`SaveContext`'s play part) is
     written from the fields the port has, zeros where it has none, and zeros for the 0x28 bytes
     past the struct;
   - as the first, with the whole tail zeros;
   - the fields the port doesn't use kept as raw bytes, carried from load to save: a smaller
     change, but the save half typed.
5. **Erase and copy** (`Sram_EraseSave`, `Sram_CopySave`: no caller without the file select):
   - *(recommended)* ported whole with their tests, and used by an `ootx sram` command (list the
     slots, erase N, copy N to M, verify), a stand-in for the file select's other screens;
   - ported whole with their tests, no caller;
   - left out (less than the whole of `z_sram.c`).

**The debug start and the exit run:** a preset `deku-tree-save` (inside the Deku Tree at its
entrance, the slingshot owned on no button, two hearts), and `Route::Save` (`--script save`) in one
sandbox run on an SRAM in memory: Start, the cursor to the slingshot, C-Left, B (the prompt turns
in), A on Yes (the save, the menu closes), then the reset: file 2 loaded, and the run checks the
state restored: entered at `ENTR_DEKU_TREE_0` (`savedSceneId` was the Deku Tree), the slingshot on
C-Left, three hearts (`Sram_OpenSave`'s floor), the slot and its backup the same bytes with a good
checksum. The golden `save` (trace and final image), with screenshots `save_prompt` (the prompt in,
the cursor on Yes) and `save_prompt_turn` (the page halfway in), and the bake-coverage test
extended to the save prompt.

### What was built

**`z_sram.c`** (`oot_game::sram`, ADR 0049), for this ROM's branches, function by function:
`Sram_OpenSave` (the entrance by the saved scene, `sDungeonEntrances`; three hearts; Zelda's letter
taken back without the lullaby; an adult's Master Sword; the spoiled trade items,
`gSpoilingItems`), `Sram_WriteSave`, `Sram_VerifyAndLoadAllSaves` (a bad file from its backup, a
bad backup a new save, file 1 the debug save), `Sram_InitSave`, `Sram_EraseSave`, `Sram_CopySave`,
`Sram_WriteSramHeader`, `Sram_InitSram`, `Sram_Alloc`; `gSramSlotOffsets`, `sSramDefaultHeader`;
`Sram_InitNewSave` and `Sram_InitDebugSave` ported in place (`SaveContext::init_new_save`,
`init_debug_save`), under `SaveContext::new` and `debug`.

**The save's bytes:** `SaveContext` gains every field of `Save` it lacked (`totalDays`,
`bgsDayCount`, `newf`, `n64ddFlag`, `ocarinaGameRoundNum`, `fw`, `highScores`,
`worldMapAreaData`, the scarecrow's songs, `horseData`, the checksum, the `unk_*` pads) and the
header's `soundSetting` and `zTargetSetting`; `save_bytes`, `slot_bytes` and `read_save` lay it out
field by field (one visitor both ways, `save.h`'s offsets checked), the slot's tail from the fields
the port has.

**The SRAM** is an image on the play state (`PlayState::sram`, carried by `reinit`); the apps bind
it to the save file (`oot_game::pack::save_path`, `eng_asset::write_file_atomic`, written after
each save).

**The file select's load stood in for** (`oot_game::file_select`): the title's and file select's
start (`Sram_InitSram`, `Sram_Alloc`, `Sram_VerifyAndLoadAllSaves`), `FileSelect_LoadGame`,
`MapSelect_LoadGame` for file 1, the name entry's new file (`Sram_InitSave`, named "LINK");
`SaveContext::file_select_new` goes through it. `PlayState::console_reset` (F5, `Task::Reset`).

**The pause menu** (`z_kaleido_scope.c`): B's three branches in `PAUSE_STATE_MAIN`,
`PAUSE_STATE_SAVE_PROMPT` whole, `KaleidoScope_DrawPages`' prompt page for both prompts (the save
page's tiles, `SAVE_TEXS`; the message, cursor and Yes/No until saved). The game over's Yes calls
`Sram_WriteSave`.

**The bake:** `gPauseSave10ENGTex` (pack format 26).

**The apps:** the game's and the sandbox's `--file N` (with `--new-file`), `--sram PATH`; the
window's F5 and its write-through; the sandbox's `--sram` image written at the end of a run, the
reset inside a route, the trace's save prompt fields and the reset's frame; `ootx sram` (`list`,
`verify`, `erase N`, `copy N M`); `golden.py`'s `{sram}` output.

**The debug start and run:** the preset `deku-tree-save`; the debug starts are file 2 ("ZELDAZ")
on an SRAM in memory; `Route::Save` (`--script save`: Start, the cursor to the slingshot, C-Left,
B, Yes, the menu closed, the reset, file 2 loaded).

### Results

**Tests.** `cargo test --release --workspace` (`target/game24`, `OOT_DATA_DIR=out/data23`): 706
passed, 0 failed, 1 ignored (683 before); 24 are new, with their expectations from the C, and
5b-1's `b_logs_the_save_prompt_and_the_menu_stays` is gone (B opens the prompt now):
- **`oot_game` `sram::tests`** (17): the slots' offsets and sizes; a new save's bytes at
  `save.h`'s offsets (the name, hearts, Link's house, the equips, the keys at -1, the Water
  Temple's switch, `infTable[29]`, Epona); `clear_info` zeroing every `SaveInfo` byte; a save read
  back as written (both ages, -1 entrances); the checksum (its own halfword left out, wrapping);
  `Sram_WriteSave`'s slot and backup with the tail; a fresh SRAM's three empty files; an SRAM of
  0xFF rebuilt (file 1 the adult debug save, as `linkAge` was cleared); a bad file restored from a
  good backup with checksum 0, put right by the next save; `Sram_OpenSave`'s entrances and fixes;
  `Sram_InitSave` (the whole SRAM written, the tail untouched; file 1 the debug save);
  `Sram_EraseSave` failing its checksum on the next boot, which rebuilds it; `Sram_CopySave`;
  `Sram_InitSram`'s header, settings, language and D-Right ramp; `SLOT_OCCUPIED`'s or; the stand-ins'
  errors, and a new file equal to `SaveContext::file_select_new`'s.
- **`oot_actors --test save`** (7): B opening the prompt (`NA_SE_SY_DECIDE`, the buttons, the
  pitch 39.25 a frame to -628, L and R out by 40); No, B and Start closing it unsaved (8 frames of
  20 to -468, the resume, the buttons back); Yes saving (`NA_SE_SY_PIECE_OF_HEART`, the flags, the
  slot and backup), 2 frames saved and the closing, A in `SAVED` closing at once; the page's tiles,
  message, cursor and choices at `sVtxPagePromptQuadsY` (not the game over's `YREG`s), nothing
  after saving; every quad baked; the game over's Yes writing file 1, which loads with three
  hearts; the exit's run.
- **`oot_game` `save::tests`**: `the_file_selects_new_file_starts_the_opening` passes unchanged
  through the real path. **`kaleido::tests`**: the bakes count 76 page tiles.

**The exit run** (`Route::Save`, `--script save`, `deku-tree-save`): `menu_opened` 52,
`cursor_on_item` 53, `item_equipped` 64, `save_prompt` 73, `saved` 74, `menu_closed` 87, the reset
before frame 88, `loaded` 119: Link at `ENTR_DEKU_TREE_0` with three hearts (from two) and the
slingshot on C-Left; the SRAM's 3 writes (the file and its backup, then the reset's header).

**The goldens.** Against 5b-2's build (`target/game23`, on data22: 105/105 identical, its outputs in
`out/golden_base5c`): every hash the same bytes. New: `save` (the trace, the loaded file's
screenshot, the final SRAM image `save/sram.sra`), `save_prompt_turn` (frame 69), `save_prompt`
(73), each the same bytes over two runs. Logged in [golden/README.md](../golden/README.md): 110
hashes, 81 cases.

### Decisions

- **[ADR 0049](adr/0049-saving.md):** `z_sram.c` on an SRAM image the play state carries, the
  apps binding it to `out\saves\<ROM SHA-1>.sra`; every `Save` field, laid out field by field; the
  file select's load stood in for (`--file N`, `--new-file`, file 1 through the map select), the
  debug starts on an SRAM in memory, F5 the console's reset; the save prompt whole; `ootx sram`;
  the faithful bugs; pack format 26.
- **A debug start is file 2** ("ZELDAZ", `fileNum` 1), as the file select's new file is in this
  port: file 1 is the map select's in this debug ROM, and its load skips `Sram_OpenSave`'s
  entrance.
- **A fresh image is zeros:** every checksum holds, three empty files.

### Known gaps

- **No title screen or file select** (BACKLOG #21): names are always "LINK"; the options' settings
  can't be changed; F5 isn't quite the console's reset (BACKLOG #22).
- **The slot's tail** has zeros where the port lacks `SaveContext`'s fields (the timers, the magic
  meter's state, the minigames ...), and for the 0x28 bytes past it; nothing reads them back.
- **Logged:** the scarecrow's songs copied out to the ocarina (`Sram_OpenSave`); the header's
  settings other than stereo, "Switch" and English.
- Still logged from 5b: the equipment and quest pages' and the world map's contents, the debug
  inventory editor (L).

### Fixes found while building

- **A debug start's save loaded as an empty file:** `SaveContext::new` is `Sram_InitNewSave`,
  whose `newf` is empty; a debug start stands for a file the file select made, so it carries
  "ZELDAZ".
- **`--file` with an empty file started a new save instead**, and the sandbox then wrote that over
  the image it had read; the load is tried up front now, and a failed start keeps the image.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-save.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-save.bat
```

### By hand

Enter is Start, WASD the stick, J the C-Left button, E is B, Space A, F5 the console's reset.

`game-save.bat` (or menu 77), a debug start (nothing reaches your save file):
1. **Equip:** Enter, then WASD to the slingshot, J: it flies to C-Left.
2. **The prompt:** E: a chime, the B and C buttons dim, and the page tips back while the
   save prompt turns up behind it, "Would you like to save?" with a green glow on Yes, "(A) to
   Decide" below.
3. **No:** D moves the glow to No; Space: the pages swing down and the game goes on. Try E (B) and
   Enter at the prompt too: the same, nothing saved.
4. **Yes:** Enter, E, Space on Yes: the heart-piece chime, and the menu closes by itself a moment
   later.
5. **The reset:** F5: the Deku Tree's entrance again, "Inside the Deku Tree", three hearts (you
   had two), the slingshot on C-Left.

`game-file.bat` (or menu 79), your save file (`out\saves`):
1. **A new file:** `game-file.bat 2 --new-file` (an empty file 2): the opening plays; walk a little,
   then Enter, E, Space on Yes.
2. **Loading:** close the game, `game-file.bat 2`: Link's house (a save outside a dungeon starts
   there for a child). F5 does the same without closing.
3. **The file tools:** `ootx sram` lists the three files; `ootx sram copy 2 3`, then
   `game-file.bat 3`.

`game-game-over.bat` (or menu 74): Yes at "Would you like to save?" now saves (to the debug start's
SRAM in memory), then "Continue playing?".
- **What to report:** the prompt's turn in and out and its speed, the glow, the sounds, the menu
  closing after Yes, the reset's feel, and anything a loaded file has wrong.

## Milestone 6: Gohma

**Status:** decided (2026-10-09). The user chose:
- three parts: 6a Queen Gohma (`Boss_Goma` whole with her intro and death, `Item_B_Heart`), 6b the
  blue warp, 6c the run through the Deku Tree;
- her decay drawn through per-draw images keyed by the texture's source address (ADR 0048
  extended), her six textures' texels in the pack, pack format 27;
- the warp leading to Kokiri Forest's first cutscene, played to its terminator (`Door_Warp1`'s
  destination warp and Player's start mode 2 ported); the rest of the chain to the backlog;
- short goldens from debug starts (the fight, the warp, room 9 into the boss room), and the full
  run from the Deku Tree's start as one test that checks events, not a golden.

### The survey

Line counts are the decomp's (`52a510f`). None of the three files has a version branch, so for
`Boss_Goma` and `Item_B_Heart` this ROM's branches are the whole file; `Door_Warp1` branches only
on its params. "Port today" is `3fdac7f` (milestone 5c).

**`Boss_Goma`** (`z_boss_goma.c`, 2,175 lines; 1,660 in functions, the rest the collider's 13
elements, `sClearPixelTableFirstPass` and `SecondPass` (256 bytes each), `sDeadLimbLifetime` (100),
the init chain). Nothing of it is ported; `En_Goma`'s boss side waits for it (milestone 3b).

| C | Lines | What |
|---|---|---|
| `BossGoma_Init`, `_Destroy`, `_SpawnChildGohma` | 28, 6, 6 | Health 10, immovable, upside down at y -300 (the ceiling), `SetupEncounter`; with the room cleared she's killed at once and spawns the warp at (0, -640, 0) and the heart at (141, -640, -84) |
| The 18 `BossGoma_Setup*` | 128 | Each action's animation and timers (`Rand_S16Offset` in four of them); `SetupDefeated` stops the music |
| `BossGoma_Encounter`, `_SetupEncounterState4` | 305, 40 | **The intro**, her own sub camera: Link held at the room's entrance (150, 350) (mode 8), the camera from the ceiling's centre zooming on him; frame 176 the slab (`Door_Shutter` `SHUTTER_GOHMA_BLOCK`) falls behind him and the lights change (setting override 3, then 4); 190 he turns (mode 2); 228 control back; then until he has looked at her (in view 16 frames, `projectedPos`), she wanders the ceiling; then the eye roll, her run, the drop to the floor (dust, sound, rumble), the cry, **"Queen Gohma"** (`TitleCard_InitBossName`, the first time only), the boss music, `EVENTCHKINF_BEGAN_GOHMA_BATTLE`, the camera back. With the flag already set (a second try) the door and look-at parts are skipped |
| `BossGoma_Defeated` | 271 | **The death:** her sub camera circling, Link pulled 100 in front of her (mode 1); bubbles every 8 frames; from 1,080 the dust and fragments at random limbs (`@bug (game)`: index 0, never written, is the origin), the death cry, **her textures erased** pixel by pixel (`BossGoma_ClearPixels`, 4 a frame, two passes; `@bug (game)`: progress 256 reads one byte past the first table, which is the second's 1, and clears the first pixel of the next texture), the room's ambient and fog flashing blue (`adjAmbientColor`, `adjFogColor`); at frame 1,001 her limbs break off as `En_Goma` pieces (params 100 + lifetime); the boss-clear music; the heart spawned at her; the camera back to the main one's view; the warp spawned at a random spot (up to 10,000 `Rand` tries) and the room cleared; she shrinks away |
| The 9 floor actions | 217 | `FloorMain` (towards Link while patient (200 frames), else away; a wall: climb), the attack (posture within 150, the lunge, quake, rest), stunned (nut 40 frames, seed 90, struck down 150), damaged, the landings |
| The ceiling and falls | 168 | `WallClimb`, `CeilingMoveToCenter`, `CeilingIdle`, preparing (70 frames, red eye: the window to shoot her down) and laying her three eggs (`En_Goma` 0 to 2), `FallJump` once they're dead, `FallStruckDown` |
| `BossGoma_UpdateCeilingMovement` | 38 | Her steps on the ceiling: 5 fragments a step (`Effect_Ss_Hahen`) |
| `_UpdateEye`, `_UpdateHit`, `_UpdateTailLimbsScale`, `_UpdateMainEnvColor`, `_UpdateEyeEnvColor`, `_Update` | 64, 49, 20, 30, 10, 53 | The eye: closes when Link shoots (`unk_A73`, which she clears) or at random (every 16 frames, 30%), open and red while she attacks; **the hit:** only on the eye, only while open: on the ceiling she falls; stunned, a sword's damage (`CollisionCheck_GetSwordDamage`: the Kokiri Sword's slash 1, its jump slash 2); patient on the floor, a seed or nut stuns her. The colours |
| `_OverrideLimbDraw`, `_PostLimbDraw`, `_EmptyDlist`, `_NoBackfaceCullingDlist`, `_Draw`, `_PlayEffectsAndSfx` | 88, 50, 10, 13, 19, 17 | Env colour on every limb, the eye's (random while invincible: draw-time `Rand`) and the iris's apart; the eyelids' and iris's rotations, the iris and tail scaled; limbs hidden when closed or broken off; segment 8 culling or not; the post draw sets her focus, claw and tail points, the colliders' spheres, the dead limbs' points, and **spawns the broken-off pieces** |

**`Item_B_Heart`** (`z_item_b_heart.c`, 117 lines; 67 in functions): the heart container, bobbing and
growing to 0.4, `GI_HEART_CONTAINER_2` offered within 30/40, collectible flag 0x1F; drawn
translucent when a `Door_Warp1` is behind it. Not ported. Its mesh is baked already as the
get-item draw `GetItem/12/0` (`GID_HEART_CONTAINER`).

**`Door_Warp1`** (`z_door_warp1.c`, 1,064 lines; 950 in functions). Not ported. What this ROM's
Deku Tree reaches:

| C | Lines | Reached |
|---|---|---|
| `_Init`, `_Destroy`, `_SetupAction`, `_ChooseInitialAction`, `_Update`, `_Draw` | 23, 12, 3, 25, 9, 28 | Yes |
| `_SetupWarp` | 89 | Its `WARP_DUNGEON_CHILD` (0) and `WARP_DESTINATION` (6) cases |
| `_WarpAppear`, `_PlayerInRange`, `_ChildWarpIdle` | 33, 13, 16 | Yes: the blue warp grows in (sound, light rays), and Link within 60 starts one-point 9703 and mode 10 (walk to its centre) |
| `_ChildWarpOut` | 62 | Its Deku Tree branch: Link floats up (his gravity 0.1), 100 frames on: `EVENTCHKINF_07` and `_09`, `Item_Give(ITEM_KOKIRI_EMERALD)`, `ENTR_KOKIRI_FOREST_0` with cutscene 0xFFF1 (a second time: `ENTR_KOKIRI_FOREST_11`), the slow white fade |
| `_DrawWarp` | 101 | Yes: two rings of `gWarpPortalDL`, each its own matrix (segments 9 and 10), scrolling |
| `_Destination`, `_DoNothing` | 19, 2 | Only if Kokiri Forest's arrival plays: its cutscene layer places a `Door_Warp1` 6 |
| The adult warp, the crystals, Ruto's warp, the clear-flag warp, `func_8099B020` (`WARP_UNK_7`) | about 530 | No: logged |

**The boss room** (`ydan_boss`, `ENTR_DEKU_TREE_BOSS_0`): two rooms; Link enters room 1 standing
(start mode 13) at (321, -640, 772); room 1 holds Gohma and 8 bushes, room 0 the way back (exit 1,
`ENTR_DEKU_TREE_1`). Its bg camera is `CAM_SET_NONE`, so the room's camera is the normal one; the
cutscenes are her sub camera, set each frame (`Play_SetCameraAtEye`). Already in the port: the
scene and its pack records (a test enters it, `doors.rs`), `Door_Shutter`'s Gohma slab (its quake
goes to the main camera where the C shakes her sub camera: a hook to replace), the manual
cutscenes, sub cameras, Player's modes 1, 2, 7, 8, 10 and 11, the lights' overrides and
adjustments, point lights, every effect and combat helper she calls, `Effect_Ss_Dust`
(`func_8002836C`), the white fades.

**Missing for her, beyond the three actors:**
- an actor driving its own sub camera (the plumbing is there; she's the first), `SUB_CAM_ID_DONE`;
- `TitleCard_InitBossName` and the boss card's sprites (`gGohmaTitleCardTex`, IA8 128x40 for
  English, drawn as 32 rows then 8);
- one-point 9703 (its keyframes are in the pack already);
- `NA_BGM_BOSS`, `NA_BGM_BOSS_CLEAR`, the `NA_SE_EN_GOMA_*` boss sounds, `NA_SE_EV_WARP_HOLE`,
  `NA_SE_EV_LINK_WARP`;
- `En_Goma`'s writes into her `childrenGohmaState` (a logging hook today).

**The draw:**
- **Her skeleton** is one baked mesh skinned by bones, as every skeleton is. Per-limb colours,
  hidden limbs, the iris and tail scale and the two segment-8 lists all have patterns already
  (the eye and iris baked apart as the larva's body is, a hidden limb's bone zeroed, a scale
  multiplied onto its bone after posing, one bake per segment-8 list). No engine change.
- **Her decay** writes zeros into six of `object_goma`'s textures (`gGohmaBodyTex`,
  `ShellUndersideTex`, `DarkShellTex`, `EyeTex` 16x16, `ShellTex`, `IrisTex` 32x32 RGBA16, one after
  another in the file), and her broken-off pieces draw with the same textures. The engine's one
  per-draw image (ADR 0048) replaces every texture of a draw with one image, so this needs a
  decision (below).
- **The warp's portal** loads two matrices from segments (9 and 10), which no bake does yet: a bake
  segment bound to a bone, the two matrices passed as the draw's bones. Its combiner blends its two
  scrolled tiles by `LOD_FRACTION`, which the shader takes as 0; to be checked against the
  hardware's value with texture LOD off.
- **The heart** is the get-item bake, in the opaque or the translucent list.

**The pack:** `object_goma`'s skeleton and animations, `object_warp1`'s portal list and
`object_gi_hearts`' lists are in it; new are the bakes (Gohma's skeleton with segment 8 bound,
cull and no-cull; her eye and iris; her limb lists as `En_Goma`'s pieces; the warp's portal with
its matrices; the boss card's two sprites) and, for the decay, her six textures' texels where the
game can read them. **Pack format 27** (`out/data24`).

**The way to her** (the exit's run):
- The travel test (`--test travel`, 7,943 frames) already walks from room 0's top floor through
  every room to room 9's door to room 11, and stops there.
- Room 11 has no actors: its floor, just past the door at y -1880, is exit 2 (a floor with
  `floor_effect` 2: the exit sets the respawn point and voids out, `respawnFlag` -2). Walking onto
  it is ported (`Player_HandleExitsAndVoids`) but no test has stepped on such a floor yet.
- MQ's Deku Tree has no small key and no boss door; nothing else is missing on the way.

**After the warp:** cutscene 0xFFF1 is Kokiri Forest's layer 5,
`gKokiriForestKokiriEmeraldPart1Cs`: Link arrives by blue warp (start mode 2, not ported) beside a
`Door_Warp1` 6 before the Deku Tree, who talks (0x1024 to 0x1027, a choice) with Navi; its
terminator goes on to the castle town's cutscene map (Ganondorf on his horse), the world's
creation in four more scenes, back to Kokiri Forest for the Triforce (layer 4) and the Deku Tree's
death (layer 6, with `Demo_Effect`), and ends at `ENTR_KOKIRI_FOREST_11`. The port has
`Bg_Treemouth`, Navi's cues, `Object_Kankyo` and the Deku Tree's death misc command; not the other
scenes' actors or `Demo_Effect`.

### The proposal

**The split.** The cutscenes are `Boss_Goma`'s own actions (`Encounter`, `Defeated`), and the fight
starts at the end of `Encounter`, so "the cutscenes" don't split off from her. Proposed:
- **6a, Queen Gohma:** `Boss_Goma` whole (the intro and the death are hers), her draw and decay,
  `Item_B_Heart` whole (67 lines, spawned by her death), the boss card, the music and sounds, the
  slab's quake on her camera, `En_Goma`'s hook replaced. `Door_Warp1` spawns as a placeholder.
  Debug start: `game-gohma.bat` in the boss room (`deku-tree-gohma`: sword, shield, the slingshot
  and nuts on C), `game-gohma.bat again` with her battle begun (the short intro).
  **Exit:** from the boss room's start, the intro, the fight (seeds into her red eye, jump
  slashes), the death, the heart taken: `Route::Gohma`, the golden `gohma`.
- **6b, the blue warp:** `Door_Warp1` for this ROM's Deku Tree (as decided below), one-point 9703,
  the slow white fade, `Player` start mode 2 if Kokiri Forest's arrival is ported. Debug start:
  `game-warp.bat`, the boss room cleared (Gohma's init spawns the warp and heart).
  **Exit:** into the warp: the float, the emerald, `EVENTCHKINF_07` and `_09`, Kokiri Forest:
  `Route::BlueWarp`, the golden `blue_warp`.
- **6c, through the Deku Tree:** room 11's exit floor, the travel test carried on into the boss
  room, Gohma and the warp. **Exit:** the run from the Deku Tree's start to Gohma's defeat (shape
  below).

The other decisions (the user's choices are in the status above):
- **The decay:** per-draw images keyed by the texture's source address *(chosen)*; or her mesh
  baked in parts, one per texture, each with ADR 0048's one image; or the decay logged.
- **Where the warp leads:** to the end of Kokiri Forest's first cutscene *(chosen)*; or to the
  warp's fade only; or the whole chain to the Deku Tree's death.
- **The run's shape:** short goldens from debug starts and the full run as a test *(chosen)*; or
  the full run (about 11,000 frames) as a golden; or short goldens only.

## Milestone 6a: Queen Gohma

**Answer:** done. Queen Gohma is fought as in the game. Link walks in and her intro plays: the
camera from the ceiling closes in on him, the slab drops behind him, and she waits on the ceiling
until he has looked at her. Then her eye rolls, she runs, drops to the floor, and "Parasitic
Armored Arachnid GOHMA" shows with the boss music. In the fight:
- a seed or a Deku nut into her eye while it's red stuns her, and the Kokiri Sword hurts her
  then, a jump slash twice as much as a slash;
- on the ceiling, a seed while she prepares her eggs knocks her down, stunned for longer;
- otherwise she lays three eggs, and jumps down once their larvae are dead.

At no health her death plays: her camera circles her, the room flashes blue, her textures are
erased bit by bit, her limbs break off, and she shrinks away. The heart container she leaves gives
a fourth heart. The exit holds; its run is the golden `gohma`, with `gohma_title` and
`gohma_decay`.

The pack is format 27, in `out/data24`. Decisions are in [ADR 0050](adr/0050-queen-gohma.md) and
[ADR 0051](adr/0051-textures-replaced-by-source-and-the-object-ram.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 80 to 82):
- `test-gohma.bat`: the milestone's tests (then 3b's larvae, the slab's and the title card's);
- `game-gohma.bat`: her room, a debug start (`deku-tree-gohma`); `game-gohma.bat again` with her
  battle begun (`deku-tree-gohma-again`);
- `sandbox-gohma.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** 6a is done; 6b (the blue warp) and 6c (the run through the Deku Tree) are
next.

### What was built

**`Boss_Goma`** (`oot_actors::boss_goma`, ADR 0050), whole, function by function:
- **her intro** (`BossGoma_Encounter`, `_SetupEncounterState4`): the trigger at the room's entrance,
  her own sub camera set every frame, Link held, turned and freed through Player's cutscene modes
  (8, 2, 7, 1), the slab (`Door_Shutter` `SHUTTER_GOHMA_BLOCK`, her child) at frame 176, the lights'
  overrides, the look-at check on her `projectedPos`, the eye roll, the run, the drop, the title card,
  `NA_BGM_BOSS`, `EVENTCHKINF_BEGAN_GOHMA_BATTLE`, the main camera written back;
- **the fight:** every floor and ceiling action, `BossGoma_UpdateEye` (Link's shot closing her eye,
  her random blinks, closed while a child lives), `_UpdateHit`, the colours, the tail's swell, her
  three eggs (`En_Goma` 0 to 2) and their larvae's deaths written back into her `childrenGohmaState`
  (the 3b hook replaced);
- **her death** (`BossGoma_Defeated`): her camera circling, Link pulled before her, the bubbles,
  dust and fragments at her limbs, the blue flashes (`adjAmbientColor`, `adjFogColor`), the decay,
  the 20 pieces at frame 1001, `NA_BGM_BOSS_CLEAR`, the heart, the warp (a placeholder until 6b) and
  the room cleared, the shrink;
- **her draw:** one skeleton bake with her env colour, her eye and iris apart (the eye's random
  colours while she's invincible made in `draw_update`), the eyelids and iris turned, the tail and
  iris scaled, broken-off limbs hidden, segment 8's two lists (two bakes of each); her post draw's
  points, spheres and pieces once per game frame.

**Her decay** (ADR 0051): textures carry the address they were loaded from
(`TextureImage::source_addr`); a draw can replace textures by that address
(`DrawParams::texture_images`); the bytes the game writes into a loaded object are play state
(`ObjectContext::written`). `BossGoma_ClearPixels` writes into her six textures' region as the C
indexes it; her draw and her pieces' pass the region's textures in place of the baked ones.

**Elsewhere:**
- `Item_B_Heart` (`oot_actors::item_b_heart`), whole: its growth, bob and spin, `GI_HEART_CONTAINER_2`,
  its flag; drawn translucent with a warp behind it.
- `TitleCard_InitBossName` and `TitleCard_Draw`'s second block; the boss name's two sprite bakes.
- Player's jump slash, pulled forward: `Player_ActionHandler_10` whole, `func_8083BA90`,
  `func_8083BBA0` (from a jump), `Player_Action_80844AF4`.
- The slab's quake on her sub camera (4a's hook replaced).
- `NA_BGM_BOSS`, `NA_BGM_BOSS_CLEAR`, the `NA_SE_EN_GOMA_*` boss sounds, `NA_SE_EV_WARP_HOLE`,
  `NA_SE_EV_LINK_WARP`.
- The presets `deku-tree-gohma` and `deku-tree-gohma-again`; `Route::Gohma`.

### Results

**Tests.** `cargo test --release --workspace` (`target/game25`, `OOT_DATA_DIR=out/data24`):
722 passed, 0 failed, 1 ignored (706 before); 16 are new. New, with their expectations from the C:
- **`oot_actors --test gohma`** (13): her init (health 10, immovable, upside down at -300, the
  lights at 4, her 13 spheres); a cleared room's warp at (0, -640, 0) and heart at (141, -640, -84);
  her intro frame by frame (state 1's camera and positions, the slab at 176 with the lights at 3,
  its quake on her sub camera, the turn at 190, the hand-back at 228); a second try (state 4 at
  once, no title card, the boss music); a seed's and a nut's stun (90 and 40 frames, not with her
  eye closed or her patience gone); Link's shot closing her eye (11 frames left after the update)
  but not while it's red; a sword's damage while stunned (1 and 2, the bubbles, invincible 10
  frames, the damage animation, the defeat with the finishing blow and the music stopped); a hit on
  the ceiling (down, the crash, stunned 150 with `sfxFaintTimer` 92); her eggs (the tail at 24, 32,
  40, 48, three eggs her children a third of a turn apart, their deaths written back, the jump
  down); her death (`framesUntilNextAction` by her updates, the 20 pieces at 1001, the first pass's
  pixels by `sClearPixelTableFirstPass` and the step-256 write into the underside's first pixel, the
  heart at 271, the warp and the clear at 341, gone by 400, the boss-clear music); the heart
  container (its growth, spin, flag, not spawned again); the jump slash (5 across and 5 up,
  `NA_SE_VO_LI_SWORD_L`, its finish on landing); her bakes and their textures' sources.
- **`oot_actors --test gohma_run`** (1): the exit's run.
- **`oot_game` `title_card::tests`** (1): a boss name's two blocks; **`object_ctx::tests`** (1): the
  written RAM kept with its object and dropped with it.

**The exit run** (`Route::Gohma`, `--script gohma`, `deku-tree-gohma`): `gohma_entered` 72,
`gohma_waiting` 301, `gohma_looked_at` 383, `gohma_battle` 632, `gohma_stunned` 674, `gohma_hit`
712, `gohma_knocked_down` 1075, `gohma_defeated` 1216, `gohma_gone` 1603, `heart_taken` 1788.

**The goldens.** Against 5c's build (`target/game24`, on data23: 110/110 identical, its outputs in
`out/golden_base6`): every hash the same bytes. New: `gohma` (the trace and the end),
`gohma_title` (frame 560), `gohma_decay` (1430), each the same bytes over two runs. Logged in
[golden/README.md](../golden/README.md): 114 hashes, 84 cases.

### Decisions

- **[ADR 0050](adr/0050-queen-gohma.md):** `Boss_Goma` whole with her cutscenes as her own
  actions and her own sub camera; her draw from one skeleton bake and two limbs apart, two variants
  for segment 8; the boss title card's second block; `Item_B_Heart` on the get-item bake; the jump
  slash pulled forward; the slab's quake on her camera; a reactive exit run.
- **[ADR 0051](adr/0051-textures-replaced-by-source-and-the-object-ram.md):** textures replaced
  by their source address, and the object RAM the game writes, for her decay; pack format 27.
- **The jump slash pulled forward:** the fight is a Kokiri Sword's, and the C's jump slash (A while
  locked on) is what a player uses on her; it rolled before.

### Known gaps

- **The blue warp** she spawns is a placeholder (6b): it isn't drawn and doesn't warp.
- **The run doesn't fight her larvae:** if she lays her eggs it stops with a failure (her `Rand`
  never lets it in this run); by hand they're there.
- **Logged:** the rumble (`Rumble_Override`); the circle shadow (`ActorShadow_DrawCircle`, for no
  actor yet).
- **The eye's random colours** while she's invincible assume IDO evaluates the three `Rand`s left
  to right, as `En_Goma`'s do (the randomness debt).

### Fixes found while building

- **The run's slingshot after a stun:** locked on, C-Right aims in third person at the target, so
  the first-person aim never fired, and Link took five hits; the route aims through the lock-on.
- **The route walked slowly** into the corridor's mouth, which rises 16, and stopped there; it
  walks at full tilt.

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-gohma.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-gohma.bat
```

### By hand

`game-gohma.bat` (or menu 81). WASD the stick, Q is Z, E is B, Space A, L the slingshot (C-Right),
K nuts (C-Down), I C-Up, R the shield.
1. **The intro:** walk up the corridor: Link stops, the camera looks down at him from the ceiling
   and closes in, a slab thuds down behind him, he turns. Once free, look up at her (L, then the
   stick to aim up; or I): she rolls her eye at you, runs, drops, cries, and the title card shows
   with the boss music.
2. **Stun her:** she walks at you; within reach she rears up and her eye turns red. Q to lock on, L
   to draw (aimed at her), let go: she's stunned (blue flashing, fainting). A nut (K) when she's
   close does it too.
3. **Hurt her:** E (B) for the sword, then Q locked on and Space (A): a jump slash (or E slashes).
   Bubbles, she flashes red. After her stun she backs off, climbs a wall and crosses the ceiling.
4. **On the ceiling:** when her eye turns red, shoot it: she crashes down, stunned longer. If you
   miss, she lays three eggs: kill the larvae and she jumps down.
5. **Her death:** the camera circles her, the room flashes blue, her body loses its texture bit by
   bit, limbs break off and roll, she shrinks away. Walk to the heart: a fourth heart.
6. **Again:** `game-gohma.bat again`: no slab or wait, straight to her eye roll, no title card.
- **What to report:** the intro's camera and timing, how the stun and her red eye read, the jump
  slash's feel, the death's look (the decay, the pieces, the flashes), the heart's bob, and the
  blue warp (not there yet: 6b).

## Milestone 6b: the blue warp

**Answer:** done. The blue warp Queen Gohma leaves grows in. Link walks into it and the camera
closes in round him; he floats up in its light, and a slow white fade takes him out with the
Kokiri Emerald (`EVENTCHKINF_07`, `_09`). He arrives by blue warp before the Deku Tree, falling
from high above, and the Deku Tree's emerald cutscene, part 1, plays: his texts with Navi, then
its terminator's transition on to Ganondorf's tale. Going through the warp a second time takes
him to the forest's path instead, with no cutscene. The exit holds; its run is the golden
`blue_warp`, with `blue_warp_float` and `blue_warp_forest`.

The pack is still format 27 (`out/data24`; one more bake, the warp's portal). Decisions are in
[ADR 0052](adr/0052-the-blue-warp.md).

**Scripts** (`scripts\run`, also in `menu.bat`, 83 to 85):
- `test-blue-warp.bat`: the milestone's tests (then 6a's);
- `game-blue-warp.bat`: Gohma's cleared room (`deku-tree-gohma-cleared`), in front of the warp;
- `sandbox-blue-warp.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** 6a and 6b are done; 6c (the run through the Deku Tree) is next.

### What was built

**`Door_Warp1`** (`oot_actors::door_warp1`, ADR 0052), for this ROM's Deku Tree:
- **the child warp** whole: it grows in (`DoorWarp1_WarpAppear`: the ring and the rays' widths,
  `NA_SE_EV_WARP_HOLE`); in it, Link (`DoorWarp1_ChildWarpIdle`): `NA_SE_EV_LINK_WARP`, one-point
  9703, his walk to its centre (`PLAYER_CSACTION_10`); his float and the way out
  (`DoorWarp1_ChildWarpOut`'s Deku Tree branch: the emerald and flags, `ENTR_KOKIRI_FOREST_0` with
  0xFFF1, later `ENTR_KOKIRI_FOREST_11`; the slow white fade); its two point lights; the room's
  light adjustments reset when it goes;
- **the destination warp** (`DoorWarp1_Destination`): killed at its init unless Link arrived by
  blue warp within 100 of it, else faded in and out;
- **its draw** (`DoorWarp1_DrawWarp`): `gWarpPortalDL`'s two rings, each on a matrix the draw
  computes (`BakeSegment::Matrix`: the matrices passed as bones), scrolling, coloured by
  `temp_f0`;
- the other kinds' setups and actions logged (the adult warp and crystals, Ruto's, the clear
  flag's, `WARP_UNK_7`, the Sages' fade).

**Elsewhere:**
- `BakeSegment::Matrix` in the importer (display lists only).
- One-point 9703 in `OnePointCutscene_SetInfo`.
- Player's start mode 2 (`Player_StartMode_BlueWarp`) and `Player_Action_BlueWarpArrive`.
- `PlayState::entrance_by_name` public; the preset `deku-tree-gohma-cleared`; `Route::BlueWarp`.

### Results

**Tests.** `cargo test --release --workspace` (`target/game25`, `OOT_DATA_DIR=out/data24`):
729 passed, 0 failed, 1 ignored (722 before); 7 are new. New, with their expectations from the C (`oot_actors --test blue_warp`, 7):
- the child warp growing in (the ring to 100, the rays' widths to 120 and 232, the lights and
  offsets of its setup, then idle);
- taking Link in (`NA_SE_EV_LINK_WARP`, one-point 9703 on `CAM_SET_CS_C` with three keyframes,
  `PLAYER_CSACTION_10`, `unk_450`), and 101 frames on, the way out (`EVENTCHKINF_07`, `_09`,
  `QUEST_KOKIRI_EMERALD`, `ENTR_KOKIRI_FOREST_0` with 0xFFF1, the slow white fade, white in), Link
  rising through the fade;
- a second time (`EVENTCHKINF_07` set): `ENTR_KOKIRI_FOREST_11`, no cutscene, no emerald;
- the arrival by blue warp (start mode 2, held over the cue's start (2857, -594) as he falls,
  landed into the cutscene's mode; the layer's destination warp killed at its init);
- 9703's keyframes from the view;
- the portal's bake (24 triangles, its two rings on bones 0 and 1);
- the exit's run.

**The exit run** (`Route::BlueWarp`, `--script blue-warp`, `deku-tree-gohma-cleared`):
`warp_entered` 139, `warped_out` 292, `arrived` 372, `emerald_part1_over` 944; texts 0x1024,
0x1091, 0x1092 (a choice), 0x1027.

**The goldens.** Against 6a's (114 hashes): `gohma/shot.png` changed. That run's last frame now
shows the blue warp, where a placeholder stood; proved by taking `Door_Warp1` out of the overlays,
which brings it back to the same bytes. The other 113 are the same bytes. New: `blue_warp` (the
trace and the end), `blue_warp_float` (frame 200), `blue_warp_forest` (700), each the same bytes
over two runs. Logged in [golden/README.md](../golden/README.md): 118 hashes, 87 cases.

### Decisions

- **[ADR 0052](adr/0052-the-blue-warp.md):** `Door_Warp1` for its two kinds here, the rest
  logged; a bake segment bound to a draw's matrix; one-point 9703; Player's arrival by blue warp;
  Kokiri Forest's first cutscene to its terminator; `LOD_FRACTION` left 0.
- **The chain after part 1** (Ganondorf's tale in the castle town's cutscene map, the world's
  creation in four scenes, the Triforce and the Deku Tree's death back in Kokiri Forest, with
  `Demo_Effect`) is in the backlog (#23), as chosen.

### Known gaps

- **After part 1** the game goes on into the chain with placeholders (BACKLOG #23).
- **The warp's second tile** isn't blended in: the combiner's `LOD_FRACTION` is 0 in the shader
  (BACKLOG #24).
- **No rumble** (logged).

### How to check

```bat
scripts\run\build.bat
scripts\run\import.bat
scripts\run\test-blue-warp.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-blue-warp.bat
```

### By hand

`game-blue-warp.bat` (or menu 84). WASD the stick, Space A.
1. **The warp:** in front of you a blue ring grows from the floor, its light rising, with a
   steady hum.
2. **In:** walk into it: a chime, the camera swings in close round Link as he steps to its centre,
   then rises with him as he floats up in the light; the screen fades to white.
3. **Out:** from white, Link falls from high above the Deku Tree's meadow and lands; the camera
   shows the Deku Tree, and his texts follow (Space through them; a choice, the first is fine).
4. **After:** the cutscene goes on to Ganondorf's tale, whose scenes aren't ported: expect
   placeholders there (BACKLOG #23).
- **What to report:** the warp's look (its rings, the scroll, the light), the float's speed, the
  fade, the fall and landing, and the Deku Tree's cutscene up to the end of his texts.

## Milestone 6c: the run through the Deku Tree

**Answer:** done, and with it milestone 6 and GAME-05. Room 9's door, unbarred once its hint
scrubs' puzzle is solved, leads into room 11, whose floor is the scene's exit 2. That exit takes
Link into Queen Gohma's room. The whole Deku Tree now runs from one `Play_Init` in a test, the
travel test carried on: every room's connection, room 11, her fight and the heart container,
then the blue warp out to the Deku Tree's emerald cutscene, part 1. The short exit run, room 9
into her room, is the golden `boss_room`, with `boss_room_door`.

No port was needed: room 11 has no actors, and its exit already worked like every other floor
exit. The pack is unchanged (format 27, `out/data24`). No ADR: 6c decides nothing about how the
game is built (a chained run and a debug start's option, below).

**Scripts** (`scripts\run`, also in `menu.bat`, 86 to 88):
- `test-boss-room.bat`: the milestone's tests, then 6a's and 6b's, and the game's debug starts;
- `game-boss-room.bat`: room 9 cleared (`--clear 9`, `deku-tree-slingshot`): through room 11 into
  her room, her fight, the warp;
- `sandbox-boss-room.bat`: the exit run headless, its trace and screenshots.

**Status of the plan:** milestone 6 is done (6a, 6b and 6c), and with it GAME-05, Phase 6 of the
[roadmap](ROADMAP.md).

### What was built

- **`Route::BossRoom`** (the `boss-room` script): room 9's debug start with the room cleared
  (`start_clears`), through its door to room 11 (transition 4, `Task::OpenSlidingDoor`), onto room
  11's floor (`Task::Exit` to `ENTR_DEKU_TREE_BOSS_0`); step `boss_room`.
- **`Playthrough::for_routes`:** several routes' tasks as one run, their frame caps added up
  (the travel test's `BossRoom`, `Gohma` and `BlueWarp`).
- **The travel test carried on** (`travel.rs`): after room 9's puzzle, the three routes chained,
  each step checked against the C:
  - room 11 through the door;
  - `SCENE_DEKU_TREE_BOSS` at its corridor (room 1);
  - her room cleared once she's gone;
  - the heart's collectible flag and a heart more;
  - the Kokiri Emerald;
  - Kokiri Forest;
  - `EVENTCHKINF_BEGAN_GOHMA_BATTLE` at the end.
- **`--clear` for the game** (`oot.exe --clear 9`): rooms set cleared after `Play_Init`, before
  `--room` (`Flags_SetClear`), as `Route::start_clears` does for the routes.

### Results

**Tests.** `cargo test --release --workspace` (`target/game25`, `OOT_DATA_DIR=out/data24`):
731 passed, 0 failed, 1 ignored (729 before); 2 are new, and the travel test runs further.
- `oot_actors --test boss_room`: the exit's run. Room 9 starts cleared; room 11 is reached
  through the door; one scene change; `ENTR_DEKU_TREE_BOSS_0` at room 1; Link standing.
- `oot --test start`, `clear_starts_room_9_with_its_door_open`: with `--clear 9`, room 9's door to
  room 11 starts unbarred; without it, barred.
- `oot_actors --test travel`: the whole Deku Tree, 10,663 frames (under the 12,000 asserted).
  Room 9 is reached at 7,406. From there, counted from the chained run's start:

  | Step | Frame |
  |---|---|
  | The door | 152 |
  | Her room | 213 |
  | Looked at | 576 |
  | Her battle | 825 |
  | Defeated | 1,417 |
  | Gone | 1,804 |
  | The heart | 1,995 |
  | Into the warp | 2,056 |
  | Out | 2,209 |
  | Arrived | 2,289 |
  | Part 1's terminator | 2,861 |

**The exit run** (`Route::BossRoom`, `--script boss-room`, `deku-tree-slingshot`):
`door_opened` 251, `boss_room` 313.

**The goldens.** Against 6b's 118 hashes, all are the same bytes; the `--clear` option and
`for_routes` change no run. New:
- `boss_room`: the trace and the end, Link in her room's corridor;
- `boss_room_door`: frame 265, Link through the door into room 11.

Each is the same bytes over two runs. Logged in [golden/README.md](../golden/README.md): 121
hashes, 89 cases.

### Known gaps

- **The travel test isn't the game from the Deku Tree's entrance.** It starts on room 0's top
  floor (`Route::Shutter`'s debug start) with the room's enemies killed. Link is placed twice:
  - on the top floor for the drop to room 3 (the scripted vine climb falls short; by hand it
    doesn't, BACKLOG #17);
  - by room 9's third hint scrub to catch it.

  The rest is played, as in 4c and 5a.
- **After part 1,** BACKLOG #23 and #24 stand as in 6b.

### How to check

```bat
scripts\run\build.bat
scripts\run\test-boss-room.bat
scripts\run\test.bat
scripts\run\golden-check.bat
scripts\run\sandbox-boss-room.bat
```

### By hand

`game-boss-room.bat` (or menu 87). WASD the stick, Q is Z, E is B, Space A, L the slingshot (C-Right).
1. **Room 9's door:** face the door ahead and to the left with its bars open, and press Space to
   go through: the camera follows Link into a short dark corridor, room 11.
2. **Into her room:** walk on across room 11's floor. The screen fades and Link stands at the
   start of Queen Gohma's corridor.
3. **On:** her fight and the warp play as in `game-gohma.bat` and `game-blue-warp.bat`, one after
   the other, with nothing reloaded between.
- **What to report:** the door and the room change, room 11's look, the fade into her room, and
  anything that differs from playing 6a and 6b from their own debug starts.
