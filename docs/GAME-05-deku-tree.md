# GAME-05: the Deku Tree

**Goal:** Phase 6 of the [roadmap](ROADMAP.md): the first dungeon, the Deku Tree, as the debug
ROM has it (Master Quest), up to Gohma's defeat. Decided in
[ADR 0028](adr/0028-phase-6-master-quest-and-the-decomp-upgrade.md): gc-eu-mq-dbg stays the only
ROM, and the decomp is upgraded first, before any dungeon work.

| # | Milestone | Status |
|---|---|---|
| 1 | The decomp upgrade: an address-based name map from `2f4c25d`'s names to the new commit's, the citations migrated by it, the importer on the new layout, the pack's record names renamed (a format bump); every test passes and the goldens are the same bytes | done |
| 2 | Damage and health: Player taking damage (kinds 3 and 4, the hit while swimming, burning, the red flash), death and game over, the enemies' damage tables (`CollisionCheck_ApplyDamage`, `DamageTable`) | done |
| 3a | Combat basics: Player's guard with the shield (blocking, deflecting); `Camera_Battle1`; the effects (`EffectSs`, `z_effect.c`), `En_Dekubaba`'s included; `En_Firefly` (Keese, 7 placed) and `En_Karebaba` (withered Deku Baba, 5); drops on death | done |
| 3b | The rest of the MQ Deku Tree's enemies: `En_St` (2 placed), `En_Sw` (Skullwalltula and Gold Skulltula, 7), `En_Hintnuts`, `En_Dekunuts` and `En_Shopnuts` (3, 2, 1), `En_Goma` (eggs and larvae, 28, pulled forward from milestone 6) | done |
| 4 | Dungeon mechanics: `Door_Shutter` and small keys; switches, torches, webs; the map and compass; the `Bg_Ydan_*` actors; the crates (`Obj_Kibako2`) | |
| 5 | Items in use: Deku sticks and nuts, the Fairy Slingshot; the C buttons in full; a minimal pause menu for equipping; saving (`z_sram.c`) | |
| 6 | Gohma: `Boss_Goma` (her larvae pulled forward to 3b); the boss room's camera and cutscenes; the heart container and the blue warp. **Exit:** a scripted run through the Deku Tree to Gohma's defeat | |

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
