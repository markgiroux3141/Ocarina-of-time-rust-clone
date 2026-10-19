# The game editor's actors (stage E3)

The game's actors placed in a level: pots, bushes, signs, enemies, characters, chests,
switches. Each set up with its own settings, carried into the level's room, and spawned by the
game when the level is played. Decided and first built 2026-10-17 (ADR 0077); the user's call
to start it then, ahead of the Phase 12-13 wait set on 2026-10-13.

## A tab or a section of its own?

**A tool in the rail, like the others** (key C, beside Props), with its own palette, its own
group in the outliner and its own inspector. Not a separate mode.

- Actors live in the same space as everything else: they stand on the floors the level builds,
  beside its props, and are placed, moved, turned and raised the way props are. A separate mode
  would need its own plan and 3D views, or would hide the level you're placing them in.
- The editor's frame already does what they need: selection, undo, the outliner, the problems
  list, the plan and 3D views, Play from the cursor.
- What *does* deserve a workspace of its own comes later and isn't spatial: the behaviour of
  custom actors (a node graph needs a big canvas, see below), and possibly a board of the
  level's flags (who sets switch 0x05, who reads it). Those open as their own windows over the
  editor, reached from an actor's inspector, rather than a mode you switch the whole editor into.

## The right panel

An actor's inspector, top to bottom. Everything comes from the actor's schema (ADR 0066), so a
new actor ported is a new actor the editor can set up, with no editor code.

| Section | What's there |
|---|---|
| Header | Its name in the palette, its group, delete. Chips: the overlay (`En_Box`), its id, and every object it needs as set up |
| Warnings | Only between rooms in the game (a door: comes with E4); never placed by the game (only spawned); an object past the room's limit; in no layer; params bits nothing reads |
| Placement | Position, height (on the ground, or a height of its own), facing (a dial; not shown when the actor reads its y rotation as a setting), tilts (rotation components it uses as rotations), a name of its own for the outliner |
| Settings | One row per schema field, the control by its kind: named values a list, a range a number (a yes/no switch for 0-1), a drop or a get-item id a list of the game's names, a text id a hex number, a switch / chest / collectible flag a flag picker (each number with who else in the level holds it, "New" for the lowest free one, the actors sharing it as chips to jump to). Fields that apply only for some values of another (`when`) come and go with it; a field split into parts (`part_of`) shows its parts |
| Flags | Every flag it reads, sets or clears (switch, chest, collectible with their numbers; the room's clear flags; the save's EVENTCHKINF and others by name), what for, and the level's other actors sharing each |
| Appears | The four layers the game picks by age and time: child day, child night, adult day, adult night |
| In the game | How often the game places it, and its configurations there, most placed first: click one to take its settings |
| Raw | Params and rotation as the game stores them, editable in hex: anything the schema doesn't name is still reachable. Bits nothing reads, with a button to clear them |
| About | The module docs' first paragraph, and its Params and Flags paragraphs |

The palette (the left panel): a search, the groups as chips (Characters, Enemies, Bosses,
Objects, Items, Triggers, Doors & warps, Set pieces, Effects), and the list with how often the
game places each. Actors the game only spawns from other actors (arrows, songs' effects) are
hidden unless asked for; doors (transition actors) are listed but can't be placed yet. Under
it: what the chosen actor starts as (the game's configurations for it), and the room's objects
with how full the room's object banks are (16 besides the keeps and Link's), and extra objects
to add.

## What's stored, and how it reaches the game

- **The document** (`overworld::doc::Actor`): the overlay's name, where it stands (x, y, and a
  height or "on the ground"), and the game's own record: `params` (16 bits) and `rot` (binary
  angles; y 0 faces south, the game's +z). What's stored is what the game reads, so nothing the
  schema doesn't name is lost. Plus the layers it's in and a name of its own.
- **The export** (`level.json`'s `actors`): each at its height as built (the highest floor under
  it; the water's surface where there's no floor), with params, rotation and layers. The extra
  objects go in `game.objects`.
- **The importer** (`oot_import::level::write_scene`): a room per game layer (child day, child
  night, adult day, adult night), each with the level's actors in that layer and the object list
  they need: each actor's profile's object, then what its schema says it needs for its params
  (`ObjectNeed`, below), then the extra ones; not `gameplay_keep` or the scene's keep. Past the
  room's 16 objects the rest are cut, with a note.
- **The game** spawns them as any room's actors (`Play_Init`, the room context).

### Object needs (ADR 0066, extended)

Many actors find an object slot for themselves besides their profile's: a pot's model is the
dungeons' keep's or `object_tsubo`'s by its params' bit 8; a Kokiri child's skeleton by its type;
a townsperson's head by theirs. Without the object they wait or kill themselves. Each schema now
says which (`ActorSchema::objects`: `ObjectNeed::new("object_tsubo").when("objectIndex", &[1])`),
and the schema test checks it two ways: a module that looks up objects declares them (or says
why not), and every room of the game that places the actor loads what it declares for that
placement's params. The game's own oversights (pots in Lake Hylia and Hyrule Castle asking for an
object their room doesn't load, "Bank danger!") are listed in the test.

## Custom actors, later

Once the game's own actors are wired up, the user wants actors of their own: a scripting
language or a node-and-edge system. Kept in mind here:

- **The inspector is already generic.** It draws whatever fields a schema lists. A custom actor
  would carry a schema of its own (its fields, flags and objects), and appear in the palette and
  inspector like any other. So the "what can be set" half is solved once.
- **The document's actor names its kind by string.** A custom one can be `custom:<name>` with
  its definition in the project; nothing in the document assumes an overlay.
- **The behaviour** runs in one generic overlay (`Custom_Actor`) that interprets the actor's
  definition, the way the game's own actors run their action functions. A node graph suits the
  game's shape well: OoT actors are state machines (an action function per state, switching on
  timers, distance to Link, flags and hits), so the graph's nodes would be states and events
  (on init, each frame, Link near, talked to, hit, flag set) and actions (play an animation, move,
  show a text, set a flag, spawn an actor, give an item). Edges are the transitions. The same
  definition could be written as text later; both compile to one small form the overlay runs.
- **Its assets** (a model, animations) come through the glTF path (E9: glTF in), into an object
  of its own the room loads like any other.

That's stage E3's sequel; nothing built yet.

## Decisions (2026-10-17, after the first hand test)

1. **Models: capture what the game draws** (the user's choice of the two ways, below). Edge
   cases are fine at first: get it mostly right; the user goes through the actors and we tune.
2. **Behaviour stays the game's code until the game is done.** Much of what an actor does is
   baked into its code (Anju's pen is a box at Kakariko's coordinates, her reward the Empty
   Bottle, her lines fixed text ids picked by a count and flags; 46 ported actors branch on the
   scene). Making that general means changing the ported game code, so it waits until the main
   game phases (the whole-game outline, Phases 9 to 20) are done: then a pass looks for what to
   refactor into generic, modular parts (and that pass feeds the custom actors). Until then the
   ported actors stay as the game has them.
3. **Low-hanging customisation now**: what needs no change to an actor's behaviour. Its params
   and rotation (built), the objects it needs (built), and next its **texts**: every line an
   actor can say listed in the inspector and editable, as a text override (below).

### Models in the views: capturing what the game draws

Two ways were weighed: the glTF characters of ADR 0068 (`out\glb\characters`, 89 of them;
characters only, T-posed, matched to actors and their types by hand), or capturing what each
actor's own draw code emits. The capture won: it covers every kind of actor (pots, chests,
switches, set pieces, enemies, characters), with the faces, heads and colours the game binds,
for each configuration, and every actor ported later gets its model for nothing.

- **The capture** (a tool, `ootx actor-captures` or the like): for each actor and each
  configuration the game places it with (the palette's variants, plus params 0), spawn it
  headless in an empty room with the objects it needs (its profile's and its `ObjectNeed`s), run
  its init and some frames (its idle pose; one frame of it), take what its draw emits that frame
  (`DrawLists`: each `DrawCmd`'s mesh by its key, its transform and bone matrices), resolve the
  meshes from the pack as the game's renderer does (`MeshSource`), and flatten them to world
  triangles round the actor's position: textured, vertex-coloured, opaque and translucent. Saved
  per (actor, params, setting rotation) under `out\editor\actors\`, with a manifest of what was
  captured and what wasn't (and why).
- **In the editor**: the 3D view draws each actor's capture at its place and facing (instances of
  one mesh); the palette's rows, the "New ..." variants and the "In the game" list show
  thumbnails of them (the props' rasteriser, `overworld::thumb`); the plan shows a small
  top-down silhouette under the dot. Without a capture, the marker stays.
- **Invisible actors** (triggers, Navi's spots, song spots, sound sources, spawners) have no
  model to capture: they're drawn as their **area** instead (a translucent cylinder or box of the
  radius or size their code uses), which shows more than a model would. That needs each one's
  area: from its schema (a field) or a constant in its code; a short list, written by hand.
- **Edge cases, to tune later**: actors that draw nothing outside their home scene, their time,
  Link's age or a flag (fall back to capturing them as placed in the game, in their own scene's
  setting); actors that need Link nearby to show (put Link near); effects and particles (left
  out at first); animated actors caught mid-motion (pick the frame).

**Built 2026-10-19** (ADR 0078), with three changes to this plan after the survey: the capture is
the game binary's (`oot capture-actors`: a tool may not link the `oot` client, and the client
already resolves meshes as it plays), run by the editor in the background as Play runs the game;
the editor draws the captures with the game's own renderer (`eng_render`, the colour combiner) in
the same pass as the level, not with its texture-times-colour shader; and the thumbnails are drawn
the same way on the GPU, not by `overworld::thumb` (which can't combine env and prim colours,
faces or lights). Below, "Built (2026-10-19)".

### Texts: what an actor says

- **The lines**: each schema lists the text ids its code can show (a `texts` list, as `objects`:
  the id, when, a note: "0x5036, none of the cuccos in the pen"), checked by the schema test
  against the text ids in the module. Actors whose params hold a text id (signs, Gossip Stones,
  walk-in texts) have it already, as a `TextId` field.
- **The inspector** gets a Dialogue section: the lines in the order and under the conditions the
  code uses them, each shown as the game's text box would (colours, choices, icons), editable.
- **The override**: an edit is stored with the level (a custom level is a scene of its own, so
  the game's message lookup takes the level's text for that id first, and the real Kakariko keeps
  its own). This is the recommended scope; a project-wide table (the mod pack, E9) can come with
  sharing. It needs a text encoder beside the game's decoder (control codes for colours, pauses,
  choices, icons, the player's name), with a round-trip test over the whole message table.

**Built 2026-10-19** (ADR 0079), with three changes to this plan after the survey: no schema had a
`TextId` field (the params-held texts were ranges whose meaning named a base), so a line can be
`TextLine::field(0x300, "params")`, the base the field goes into; the readable form is the brace tags
our `msgdis.py` port already prints, with the decomp's charmap, not the decomp's C macros; and the text
box is drawn by running the game's own `MessageContext` headless and drawing its sprites with the
game's renderer. Below, "Built (2026-10-19): the texts".

### Recommendations (not asked for, worth keeping in view)

- **Warn about scene-bound actors.** Actors whose code branches on the scene (46 now) behave in a
  custom level as they do "elsewhere": Anju counts cuccos in a pen at Kakariko's coordinates. The
  inspector should say so ("plays as outside Kakariko and Impa's house"), from a list the schema
  test can keep honest (the scene ids each module compares). No behaviour change, just honesty.
- **Link's age and the time in the capture and the inspector**: show which layers an actor
  actually appears in (its code's own age and day checks), not only the layers it's placed in.
- **Captures are derived data**: rebuild them when the pack or an actor's module changes (a
  stamp per capture), never commit them (they come from the ROM, like the glb export).
- **Any actor anywhere, later** (the user's question, 2026-10-19). Many actors decide in their
  init whether they exist at all: Dampé (`En_Tk`) kills himself unless it's the graveyard, Link a
  child and 18:00 to 21:00, so in a custom level (a scene of its own) he never appears; 46 check the
  scene, others story flags or a cutscene. In the refactor pass (Decisions, 2) these checks become
  an "appears when" setting per actor, the game's rule its default (so the real graveyard plays as
  before) and a level free to change it. Actors with a job tied to their place (Dampé's path and
  dig spots, Anju's pen) need that job made settings too. Not now: noted so it isn't lost.
- **Text ids for new lines**: when custom actors come, they'll need text ids of their own; keep
  the level's override table able to hold new ids (above the game's last), not only replacements.

## Built (2026-10-17)

- `overworld::doc::Actor`, its export, `oot_import::level`'s rooms per layer with actors and
  objects (`room_actors`, `room_object_limit`), the app handing in each actor's object needs.
- `oot_game::actor_schema`: `ObjectNeed`, `ActorSchema::objects` and `needed_objects`, and
  `GI_NAMES` (the get-item ids by name); every schema's `objects`, and the two checks in
  `oot_actors/tests/schemas.rs`.
- The editor: `actors.rs` (the catalogue: `oot_actors::PROFILES`, the palette's names and groups
  in `NAMES`, the module docs, and the game's placements read from the pack on a thread),
  `app/actor_ui.rs` (markers in the plan and 3D, the palette, the inspector), the Actors tool (C)
  in the rail, the outliner's Actors group, `--select actor:N`.
- `scripts\run\editor.bat` (menu 147): builds the editor and the game into the scripts' target
  and opens it with the scripts' pack; Play runs the game built beside the editor.

Tests: `oot_import` `room_actors_take_their_ids_and_objects`; `overworld` (export)
`actors_are_exported_standing_on_the_level`; `oot` (level) `a_levels_actors_spawn_in_their_layers_with_their_objects`;
`oot_actors` (schemas) `object_lookups_are_declared`, `needed_objects_are_in_the_rooms_that_place_them`;
`overworld_editor` `actors::tests` (names for every ported actor, labels, setting rotations, the
game's placements read).

## Built (2026-10-19): the actors as the game draws them

- **The capture** (`crates/apps/oot/src/capture.rs`, `oot capture-actors [--all] [--only ...]
  [--pngs]`): every placeable actor in params 0 and in the most placed 24 of the configurations the
  game's scenes place it with, and the configurations a level asks for (`extra.json`); each in a
  fresh play state in a flat level entered by `Play_Init`, its objects loaded (the actor table's by
  its row, its schema's `ObjectNeed`s), Link 260 in front facing it, run to frame 10 (or the first
  it draws, up to 40); its draw and its children's called once more, resolved by the game's
  `Meshes` and flattened (`eng_gfx::flatten`). Drawing nothing as first placed, it's tried as if in
  that scene (`sceneId`), the other age, the other time. 1314 configurations in about 4 s.
- **The files** (`oot_game::actor_capture`): `out\editor\actors\<overlay>.bin` (an
  `ActorCaptures`: one flattened `DrawList` per configuration, with its bounds) and
  `manifest.json` (each configuration: captured, nothing drawn or failed, the setting or the
  reason, its frame, triangles and size; the capture setting's lights). Stamped with the pack's id,
  the game binary's and the module's last change; only stale ones are made again.
- **The editor** (`captures.rs`): runs the game's capture at the start and whenever the level has
  a configuration not tried yet; reads the manifest and each actor's file when needed; finds an
  actor's capture (its own, else the nearest captured configuration), its outline from above, and
  the areas of invisible actors (`actors::area`: Navi's spots, song, talk and weather spots,
  spawners; hand-written from their settings and code constants). `view3d.rs` draws each placed
  actor's capture with the game's renderer in the level's pass (`eng_render::Renderer::set_view`,
  `draw_models`), lit by the capture setting's lights, fogged as the level; picks by the capture's
  box (models before areas); shows the selected one's box and facing. The plan shows each capture's
  outline under its dot (and picks by it). The palette's rows, its "New ..." list, the inspector's
  header and its "In the game" list show GPU thumbnails (`View3d::actor_thumb`). The inspector says
  how it's drawn: its own capture and the setting, the nearest one, its area, or a marker and why.
- **The script**: `scripts\run\editor-actors.bat` (menu 148) runs the capture with `--pngs`:
  every capture drawn to a picture and a contact sheet of them all.
- **Found on the way**: the game's object table has empty duplicates of `object_md` and
  `object_ny` before the real ones; `SceneTable::object_id` and the level importer now take the
  last of a name (a level's extra `object_md` loaded the empty one).

First run: 942 of 1314 configurations captured (889 in the plain setting, 53 only as if in their
own scene), 372 drew nothing; 139 of the 175 placeable actors have a capture. The 36 without one:
the invisible ones (drawn as their areas), what only other actors spawn (arrows, nuts, songs'
effects), and characters and set pieces that need a flag, an item, a path or a cutscene (BACKLOG
#90). Wrong ones to tune: BACKLOG #91; no shadows or per-theme lights: #92.

Tests: `eng_gfx` `flatten::tests` (posed and baked, textures shared and replaced); `oot` (capture)
`actors_are_captured_as_their_own_draw_code_draws_them`; `overworld_editor`
`captures::tests::the_nearest_captured_configuration_stands_in`, `hulls_keep_the_outside_points`,
`actors::tests::invisible_actors_areas_come_from_their_settings`.

## Built (2026-10-19): the texts

- **The lines** (`oot_game::actor_schema::TextLine`, `ActorSchema::texts` and `lines`): each line's id
  (or a base and the params field or-ed into it), `when`, its condition in words and a note, in the
  order the code uses them, the ids as a custom level plays them. Filled for every ported actor by six
  helpers by group, each id checked against the decomp's C: **54 actors, 701 lines**. Every text-id
  literal the modules use is tied to a line or carries a `// schema:` note (a flag number that looks
  like an id, an id only compared, a part not ported).
- **The readable form** (`oot_game::message_text`): `decode` and `encode` (the decomp's charmap and
  the brace tags), the measure (`measure`, `layout_warnings`: four lines a box, a line to the box's right
  edge), `next_text_id`, `MessageTable::layered` (a level's texts over the game's), `preview` (the game's
  `MessageContext` typing a line out box by box), `SceneTexts`, `FIRST_NEW_TEXT_ID` (0x8000).
- **The level's texts**: `overworld::doc::Doc::texts` (`Text`: id, box type and position, the readable
  text; `text`, `set_text`), the export's `texts` in `level.json`, `oot_import::level::LevelText` and
  `scene_texts` into the record `texts/<scene file>` (`keys::scene_texts`), and
  `GameAssets::messages_for`, which `Play_Init` plays a scene with: the level's texts in its own scene,
  the game's everywhere else.
- **The editor**: `text_box.rs` (the preview's sprites drawn by `eng_render` from the pack's bakes,
  cropped to the box, a few a frame, kept), `app/dialogue_ui.rs` (the Dialogue section, between Flags and
  Appears: the lines with their conditions, each drawn as the game's box, the texts it goes on to, and
  per text the box type and position, the text unwrapped, why it won't encode, too long, back to the
  game's). Screenshots: `out/editor/screens/7-dialogue-an-edited-line.png`, `8-...too-long...`,
  `9-...back-to-the-games.png`.

Tests: `oot_game` `message_text::tests` (codes and characters, the layout warnings, a layered table,
the preview's boxes and icons, `the_games_own_messages_fit_their_boxes`, `every_message_round_trips`:
2115 of 2115); `oot_actors` (schemas) `text_ids_are_the_schemas_lines`; `oot` (level)
`a_levels_texts_are_what_its_scene_shows_and_the_real_scenes_keep_theirs`; `overworld` (export)
`texts_are_exported_by_id_and_kept_in_order`.

Left (BACKLOG #94 to #97): the mask reactions and Navi's enemy texts the port doesn't give, parts of
actors not ported, the preview's fixed box position and name, the characters that only talk in their
own scene.

## Next

1. The hand test's feedback on the models and the texts, and the tuning pass with the user (BACKLOG
   #90 to #92, #97): what each uncaptured actor needs set before its spawn, the frame, the wrong ones.
4. **Paths** for the actors that follow one (the scene's paths: E6), drawn and picked.
5. **Several selected**: move, copy and delete actors together; copy settings from one to others.
6. **The flags board**: the level's switch, chest and collectible flags in one window, who sets
   and who reads each.
7. Doors and other transition actors come with rooms and exits (E4).
8. Custom actors (above), after the game's phases and the refactoring pass (Decisions, 2).
