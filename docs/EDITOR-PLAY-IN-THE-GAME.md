# Level editor: playing levels in the real game

A plan from 2026-10-11 for the overworld editor's ▶ Play: playing a custom level the way the game plays its own scenes,
with the music and ambience, the HUD, Navi, the sky and the clock, effects, voids, saves and (later) actors, instead of
the spikes' view it uses now. It builds on [ADR 0035](adr/0035-custom-levels-from-the-overworld-editor.md), whose
"toward a general play mode" this carries out, and fits with
[EDITOR-ROOFS-STOREYS-DUNGEONS.md](EDITOR-ROOFS-STOREYS-DUNGEONS.md) (dungeon mode needs rooms, exits and actors too).

## Progress

- **2026-10-11, steps 1 to 4 (first cut), uncommitted:**
  - `eng_asset::MemSource`: records in memory, layered over the pack.
  - `oot_import::level::add_scene` gives a level the id after the table's last (0x6E) with four entrance rows from
    0x614 and its start's HUD restrictions row. `write_scene` writes its header, room and collision: the start's
    header and first room, the theme's lights, fog and sky, then the level's changes.
  - The game's `--level` (`oot::enter_level`) enters it by `Play_Init`. `oot::reload_level` reloads it: the tables are
    built again (about 30 ms), then `PlayState::reload_scene` runs `Play_Init` with the same save and audio, an
    instant transition, and Link put back.
  - `oot_sandbox --level <dir> --in-game` does the same headless.
  - The editor runs `oot`, with a ▾ menu (child or adult, the time of day, a save preset). Its Level settings have
    *In the game* (start from, music, hot, sky, clock: the document's `game`).
  - Tests: `crates/apps/oot/tests/level.rs`, `crates/tools/overworld/tests/export.rs`.
  - Left for later: the camera type, Navi's hints and the HUD rules have no selectors of their own (they come with
    *Start from*). Restrictions copy a whole row. The play options aren't saved with the document.

## Where we start from

▶ Play exports the level and runs `oot_sandbox --level <dir> --child` (`overworld_editor::app::play`). The sandbox's
`load_level` (`crates/apps/oot/src/lib.rs`) loads the export with `oot_import::level`, which already makes the game's own
records in memory: a `RoomData` (one room, one entry, OPA and XLU draw lists), a `CollisionHeader`, and a `SceneData` of
one layer. `new_play_at` then puts Link in it directly. That's the spikes' way into a scene: `Play_Init` never runs and
the play state has no `GameAssets`, so everything that hangs off them is missing:

- no music or nature ambience (nothing runs the scene's sound settings, and the audio's game side never boots with the
  game's tables);
- no HUD, no Navi, no title card, no pause menu, no save;
- no skybox, sun or moon, and the clock doesn't run;
- no keep object, so effects that draw from `gameplay_field_keep` or `gameplay_dangeon_keep` can't spawn;
- falling out respawns by the sandbox's shortcut, not `Play_TriggerVoidOut`'s fade and `Play_Init`;
- no actors, no exits.

## The idea: a custom level is a scene like any other

`Play_Init` (`PlayState::play_init_with`, `crates/game/oot_game/src/play_scene.rs`) needs only these things:

1. an entrance row in the scene table (`SceneTable::entrances`), one per layer: child day, child night, adult day,
   adult night;
2. the scene entry it names (`SceneTable::scenes`: id, file, draw config, title card);
3. the pack records `scene/<file>`, `room/<file>/<layer>/<room>` and the layer's collision.

The level's records are the same serializable types the pack stores, so the level is written as a loose folder and
layered over the pack (`eng_asset::Assets::layer`: the last source wins). The scene table's new rows are appended in
memory after `GameAssets::load` (`scenes` is a public field), so the real pack is never touched or re-imported. From
there `Play_Init` enters the level as it enters Kokiri Forest, and everything in the list above comes with it.

## Own ids, not borrowed ones

The clone isn't limited to the game's 110 scene ids. A custom level gets **its own id**, appended after the table's
last entry, and its own properties. The 2026-10-11 investigation sorted what a scene's behaviour hangs on into four
kinds:

| Where it lives | What | For a new id |
|---|---|---|
| **The scene header** (`LayerData`) | music and nature ambience (`sound`), skybox and light mode (`skybox`), the environment lights and fog (`light_settings`), the keep object (field or dungeon), Navi's C-Up hints (`c_up_elf_msg_num`), the scene camera type, spawns, exits, transition actors, paths | any value we write: free |
| **The room header** (`RoomData`) | `behavior` (type 2 is the environment: `ROOM_ENV_HOT` is the heat timer and Link's hot-air hazard, already ported), echo, time flow (`time`), skybox and sun/moon off, actors and objects | any value we write: free |
| **The collision** | surface types per polygon: floor types (lava, quicksand, voids, damage), footstep sounds, wall types, climbable, exits | already chosen per surface role in `oot_import::level::ROLES`: free |
| **Tables keyed by scene id** | the HUD's restrictions (`InterfaceTables::restrictions`: which buttons and items work there), the minimap and pause map (`MapTables`), the overworld check for the minimap (`interface::is_overworld`, ids 0x51 to 0x64) | needs a row for the new id, or the property moved onto the scene entry |
| **Code that checks for a scene** | about 45 files compare against a `SCENE_*` constant: the original game's one-offs (adult Kokiri Forest's layer, Hyrule Field's Spiritual Stones layer, the shops, the Shadow Temple's void check) | doesn't match a new id, so the level gets the general behaviour; where we want one, it becomes a property (below) |

So a Kokiri Forest-looking level with Death Mountain Crater's heat timer is just room `behavior[1] = ROOM_ENV_HOT`, with
nothing borrowed from either scene. Two limits come from the save:

- **Scene flags:** the save keeps `sceneFlags[124]` (`SCENE_FLAGS_COUNT`); the game uses 110, so ids 0x6E to 0x7B (14
  levels) save their chests, switches, clears and collectibles in the real SRAM layout. More than that needs our own
  extension to the save (a second block, or a sidecar file next to the `.sra`).
- **Dungeon items:** small keys, map, compass and boss key are kept per `mapIndex`, 19 dungeons (`dungeon_keys: [i8; 19]`).
  Custom dungeons beyond those need the same kind of extension. Not needed until dungeon mode.

For editor play only one id is needed (every custom level plays as the same "editor" scene). Exits between levels need
an id per level, so the export should carry one from the start.

## Level settings: the game's dynamics as selectors

The editor's Level settings get an **In the game** section (built 2026-10-11) of the properties in the table, with a
**Start from** picker of the game's regions. Choosing Death Mountain Crater gives the crater's dynamics: its music and
ambience, its heat, its HUD restrictions, its keep object, Navi's hints and the camera type. Each can then be changed on
its own. The theme keeps choosing the look: the lights, fog and sky stay the theme's scene's whatever the start (built
that way, so a Kokiri-looking level that's hot keeps Kokiri's light). Start from defaults to the theme's scene, so a
Zora's Domain level plays in Zora's Domain's lights, fog and music without touching anything.

The groups, roughly:

- **Sound:** the sequence (by name from the sequence table, or none), the nature ambience, whether the day's music stops
  at night (`TIMESEQ_*`, from the sound settings' spec).
- **Sky and light:** the skybox (or none, which an enclosed level picks), the light mode (time-of-day lights or the
  fixed settings), the environment settings themselves (the theme's scene's, the fog editable as the theme's fog is
  now), sun and moon on or off.
- **Time:** the clock running or stopped, and its speed (the room's time settings).
- **Hazards:** hot (the heat timer), and later cold water, the underwater timer's settings, wind.
- **Rules:** the HUD restrictions (B button, items, bottles, ocarina, warp songs, hookshot; the same bits as the game's
  table), overworld or dungeon (minimap and the pause map's page).
- **Camera:** the scene camera type (normal, fixed shop views).
- **Navi:** her C-Up hint set.

Where these live: the level document (`overworld::doc`) gets a `game` block, the export writes it into `level.json`
beside `scene`, and `oot_import::level` writes it into the scene and room headers and the new id's table rows. A
property whose behaviour today only comes from a hard-coded scene check (the last row of the table) is lifted into a
named property when a level first needs it, one at a time, never by giving the level a real scene's id.

## Actors: a tab of their own

Props stay what they are: static meshes the builder bakes into the room and its collision. Actors are a separate tab
in the editor (with their own palette and outliner), because they're a different thing: an actor id, params, a facing,
and the objects it needs, written into the room's `actors` and `objects` lists, spawned by `Play_Init` and the room
context, and reset on every reload. Some kit props have a game actor that does the same job with behaviour (cuttable
grass `En_Kusa`, bushes, liftable rocks `En_Ishi`, signs, pots), so the actors' palette offers those as actors, and a
level can use either. Placing actors is the step after this plan; nothing here needs it.

## Steps

1. **The level as records, with its own id.** `oot_import::level` gets a function that writes the level's records
   (`scene/`, `room/`, `col/`) into a loose folder beside the export (`<dir>/pack/`), with the scene header and room 0's
   header filled from the theme's scene (sound, skybox and light mode, environment lights, keep object, room behaviour
   and time). A second function appends the new scene entry, its four entrance rows (all to spawn 0) and a restrictions
   row to a loaded `GameAssets`. Unit tests: a level written and read back through `GamePack` gives the same room and
   collision; `Play_Init` on a test level places Link at its spawn.
2. **Launched through the game.** `load_assets` with `Options::level` loads `GameAssets`, layers the level's folder,
   appends the rows and sets `entrance`, so `new_play` takes its `Play_Init` path; `--at` becomes `start_at` (placed
   after `Play_Init`, as `--entrance` runs do). The `oot` binary gets `--level`. The editor runs `oot` instead of
   `oot_sandbox`, and passes child or adult, the time of day and a debug save preset (a small menu next to ▶ Play: none,
   sword and shield, slingshot and nuts). `oot_sandbox --level` keeps the spikes' view for scripted runs.
3. **Live reload.** When `level.json` changes, the folder is rewritten and the game re-runs `Play_Init` with the same
   save, the current audio (`play_init_with` takes it, so the music carries on) and Link where he stands. No fade.
   Actors and the room's state reset each time.
4. **The Level panel.** The `game` block in the document, the export and the importer; the panel with Start from and
   the groups above, ending with the hazards and rules. Each new field gets a test that it reaches the header or table
   row it's meant for.
5. **Properties lifted out of hard-coded checks**, as levels need them.

Then, outside this plan: the actors tab; exits between custom levels and the game's scenes (an id per level, exit
surfaces and the exit list); several rooms per level, which also lifts the 8192-vertex collision limit and is what
dungeon mode needs.

## Open questions and risks

- **Map data for a new id:** the minimap and the pause menu's map page index `MapTables` by scene. An editor level should
  have no map (both pages blank) until dungeon mode generates one. Check that nothing indexes past the table.
- **The scene-id restrictions table** matches `scene_id & 0xFF` against a list of bytes. Fine for ids up to 0xFE; past
  that, the restrictions belong on the scene entry instead.
- **Enclosed levels** (an outline ceiling) want the skybox off and probably the dungeon keep. Start from should pick
  that from the theme, and the theme's scene for caves (`ydan` and the like) is the natural default.
- **Reload cost:** `Play_Init` is more than the room swap (actors, the interface, the environment). Probably still well
  under a frame budget for a one-room level, but to be measured; if it isn't, reload only the room and collision and
  keep the play state, as the sandbox does now.
- **Voids:** a void floor now really fades and respawns at the entrance (`Play_TriggerVoidOut`); the respawn point is
  the level's spawn, not where Link last stood, unless the editor's "play from here" spawn is written as the entrance.
- **Pack staleness:** the layered folder is the editor's and is rewritten each export; the base pack's `is_current`
  check is unaffected.

## Hand tests (once steps 1 to 3 are in)

- ▶ Play on a Kokiri-themed level: Kokiri Forest's music and ambience, hearts and rupees, Navi, the sky, the clock moving.
- The same on a Zora's Domain-themed level: its music and lights, and the minimap blank.
- Edit while playing: the level reloads with Link in place and the music doesn't restart.
- Walk into a void floor: the fade, then the respawn as the game does it.
- Play from here: Link starts at the clicked point.
- With the sword preset: B swings, and the hit sparks and dust show.
- (Step 4) a Kokiri-themed level with Hazards set to hot: the heat timer appears and counts down.
