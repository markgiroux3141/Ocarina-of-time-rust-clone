# 0077: The actors tab: a tool in the rail, the game's own record stored, settings from the schema, object needs

- **Status:** accepted, built (2026-10-17)
- **Date:** 2026-10-17
- **Builds on:** [ADR 0062](0062-the-aim-master-quest-and-the-game-editor.md) (the aim),
  [ADR 0066](0066-the-actor-schema.md) (the actor schema), docs/EDITOR-PLAY-IN-THE-GAME.md
  (a level is a scene of its own, entered by `Play_Init`).

## Context

Stage E3 of the game editor track: the game's actors placed in a level. On 2026-10-13 the user
had put the editor's interface off until about Phase 12 or 13; on 2026-10-17 they asked to start
it, and to decide whether actors deserve a section of the editor of their own or a tab like the
others, with a right panel able to set every aspect of each actor (different actors having
different capabilities), and with custom actors (a script or a node graph) in mind for later.

## Decision

1. **A tool in the rail** (Actors, key C), with its own palette, outliner group and inspector,
   not a separate mode: actors stand on the level's floors among its props and are placed and
   moved the same way, and the editor's selection, undo, views and Play already serve them.
   Non-spatial editors (a custom actor's behaviour graph, a board of the level's flags) will be
   windows of their own, opened from the inspector.
2. **The document stores the game's own record** (`overworld::doc::Actor`): the overlay's name,
   the position (a height or "on the ground"), `params` and `rot` as the game has them, the
   layers it's in, a name of its own. The editor shows them as fields; nothing the schema
   doesn't name is lost, and the Raw section edits the bits directly.
3. **The inspector is the schema's** (ADR 0066): a control per field kind, `when` and `part_of`
   followed, flags with the level's other holders, the game's own configurations to pick from.
   A new actor ported needs no editor code; a custom actor will bring a schema of its own.
4. **The catalogue links the content crates** (a tool may, ADR 0002): `oot_actors::PROFILES` for
   the actors and schemas, the module docs read from the source for the About section, and the
   game's pack read on a thread for how its scenes place each actor (counts, configurations,
   which are only ever doors or only ever spawned). The palette's names and groups are a table
   in the editor (`actors::NAMES`), with a test that every ported actor is in it.
5. **A room per game layer**: `oot_import::level::write_scene` writes the level's room for
   each of the four layers with the actors in it, and the object list they need.
6. **Object needs in the schema** (ADR 0066 extended): `ActorSchema::objects`, a list of
   `ObjectNeed { object, when, note }`, the objects an actor finds slots for itself besides its
   profile's, by field value where one decides it. The importer lists them in the room (the app
   hands each actor's in, so the importer doesn't link the actors), the editor counts them
   against the room's 16. `oot_actors/tests/schemas.rs` checks that modules looking up objects
   declare them (or carry a `// schema:` note) and that every room of the game placing an actor
   loads what it declares for that placement; the game's oversights are listed in the test.
7. **Play runs the game built beside the editor** (`scripts\run\editor.bat` builds both into
   the scripts' target and opens the editor with the scripts' pack).

## Consequences

- A milestone that ports an actor also gives it a palette name (`actors::NAMES`) and its object
  needs; "How a milestone is done" says so through the schema test.
- Doors and other transition actors wait for rooms and exits (E4): listed, not placeable.
- The views draw actors as markers in their group's colour; their models are a later step.
- Custom actors: a `custom:<name>` kind with its own schema and a behaviour run by one generic
  overlay (docs/EDITOR-ACTORS.md); not designed further here.
