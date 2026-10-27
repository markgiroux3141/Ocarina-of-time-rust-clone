# 0094: The editor's Reference mode (the game's dungeons, a floor at a time)

- **Status:** accepted, phases 1 and 2 built (2026-10-26)
- **Date:** 2026-10-26
- **Builds on:** [ADR 0078](0078-actor-models-captured-from-the-game.md) (the game's renderer in the
  editor's 3D view), [ADR 0002](0002-crate-layout.md) (a tool may link the content crates).

## Context

Stage E5 of the level editor ([EDITOR-ROOFS-STOREYS-DUNGEONS.md](../EDITOR-ROOFS-STOREYS-DUNGEONS.md))
had built storeys, holes and stairs (slices 1 to 3) on a test level too bland to show what the
tools still lack, and designing a dungeon's tools from memory is hard. The game's own dungeons are
the reference: orbit them a floor at a time, then rebuild them with the editor's tools to find
the missing features. Master Quest's geometry (the ROM's, gc-eu-mq-dbg) is fine for that.

## Decision

1. **A Reference mode in the editor** (the top bar's *Reference*), not a separate viewer: it
   reuses the 3D view's orbit camera and the game's renderer already in it, so a later phase can
   draw the real level as a ghost over the one being rebuilt. The level being edited stays as it
   is underneath. Data in `overworld_editor::reference` (no UI, tested against the pack), the
   mode's panel and view in `app/reference_ui.rs`, with a `View3d` of its own (its own camera).
2. **Rooms from the pack, as the game draws them:** `oot_game::play_scene::SceneState::load` (the
   layer the game picks for the dungeon's age at noon), every room's entry lists uploaded once
   with `eng_render`, posed into the editor's axes; the scene draw config's segment values and
   textures (`run_draw_config`) each frame at the game's 20 frames a second, and Jabu-Jabu's site
   matrices with them. The scene's lights, no fog. No room culling: every room shown is drawn.
3. **Floors from the pause map's tables** (`MapData`, read by the importer from `z_map_data.c`):
   `floorID` names them, `floorCoordY` gives the heights `Map_Update` puts Link on each, and
   `paletteRoom` the rooms each floor's map draws. A floor shows its rooms with everything over
   the floor above's height cut away. A room on no floor's map goes on the floor its lowest point
   is on. A boss's room is its one `bossFloor`. Scenes without a map (the Training Ground,
   Ganon's Castle and Tower) have no floors: the height cut alone.
4. **`eng_render::Renderer::set_clip`:** a world-space plane past which nothing is drawn (a
   discard after the far-plane test in `fs_main`, on the posed position). The editor's 3D view
   sets it at its cut, so the game's meshes (the reference rooms, the placed actors) are cut where
   the level is. The game never sets one; with none set the shader cuts nothing (the golden
   renders are unchanged).
5. **Phase 2, what the level does:** the scene's collision from the pack, each poly sorted by what
   the game does with its surface type (`reference::Surf`, from `SurfaceBits`: the fields
   `oot_game::surface` reads, and `SurfaceType_IsHookshotSurface`'s word 1 bit 17): exit, void,
   hurts, ladder, climb, crawlspace, push / pull, hookshot, sand, slippery, else floor, wall or
   ceiling by its normal. Climbing wins over the hookshot (the Deku Tree's vines and ladders carry
   the hookshot bit too); the status bar describes every field of the surface under the pointer.
   A poly's room is the smallest room whose meshes' bounds hold its middle, so the room list and
   the floors filter the collision too. It's drawn by the editor's own pipeline (`View3d`'s
   overlays: opaque alone, translucent over the rooms, culled from behind as the game's meshes
   are), with the water boxes' surfaces.
6. **The actors:** each room's and the transition actors (the doors, with the rooms each joins),
   named by the actor table; drawn as the game draws them by the captures the editor already has
   (`oot capture-actors` captures every placement in the game's scenes, ADR 0078) in the
   catalogue's group colours, with markers to click for an actor's details. An actor the port
   hasn't got has a grey marker only.
7. **Measuring:** two clicks on what's drawn (the rooms' meshes, else the collision): the distance,
   across and up.

## Consequences

- The reference rooms and the placed actors share the game renderer's one set of globals, so a
  view with both draws the actors in the dungeon's lights (only the Reference view has rooms for
  now; phase 3's ghost will meet it).
- The pause map's floors are the game's, not the geometry's: a floor's cut is the next floor's
  `floorCoordY`, which may sit under some of that floor's ledges. The height cut slider is there
  for those.
- Phase 3 (tracing: the real level as a ghost in 3D and a backdrop in the plan) follows.
- The surface names past what Player's code shows (floor types 6, 8, 11; wall type 1; floor
  properties 7 to 9) are numbers in the description, not guesses.
- The Reference view renders headless in a test (`render_reference_views`: an egui render state
  without a window), so its look can be checked without opening the editor.
