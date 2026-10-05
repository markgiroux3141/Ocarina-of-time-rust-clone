# 0036: Kit pieces from Kokiri Forest, and props in custom levels

- **Status:** accepted, built (2026-10-05)
- **Date:** 2026-10-05
- **Builds on:** [ADR 0035](0035-custom-levels-from-the-overworld-editor.md) (custom levels, the
  export, the Kokiri texture library).

## Context

Custom levels need Kokiri Forest's buildings and features. They have to come from the clone's own
extract of spot04, without Blender at build time, and without committing ROM data. Some are whole
pieces of spot04:
- the houses and the shop;
- stumps, stepping stones, the hedge;
- the log tunnels of the Lost Woods and Hyrule exits;
- the crawlspace, the vine patch and the waterfall.

Others only make sense generated along a line in Kokiri's textures: the dirt path, fences and
bridges. A survey of spot04 (in Blender and the game) found:

- **The dirt path is decals.** 22 flat quads lie on the ground in three kinds: strips, with u
  across about 160 and v along, repeating every 170; junctions; and end caps. Their 64 x 64
  textures are white blotches with soft alpha, tinted yellow-brown (prim 9B8C34 at 70%). The floor
  under them is collision surface 13, which plays dirt footsteps, and has exactly their outline.
  `KOKIRI_ROLES` had mislabelled their textures `water_foam` / `water_ripple`.
- **The ground is two textures** blended 50/50 (`(T1 - T0) * EnvAlpha + T0`): the camo every 400 units
  and a detail texture every 50 (its own UVs). The glb has the detail's image, but its material
  named only texture 0, so our levels' ground was the camo alone, blurry next to the game's.
- **The two log exits are one model** (41 triangles), joined to the terrain. So is the crawlspace:
  40 wide, 27 high and 320 long, with an arch at each end, through the ridge between the plateau
  and the training area.
- **Fences** are 40-tall quads with a post every 40 drawn into the texture. The lattice fence is
  120 tall. The plank walkways are flat decks 40 wide.
- **Each house's door shadow is a decal** in front of its doorway.

## Decision

- **A committed manifest, a cut kit in out/.** `crates/tools/overworld/kit/kokiri.json` holds
  definitions only. For each piece:
  - its source: a box in the scene (builder axes, glTF's (x, -z, y)), optionally limited to room
    meshes or to material roles;
  - its origin: "base", "top", "door" or a point;
  - its facing: "door", "normal" or a direction;
  - its scale limits, its door (exit and entrance), its opening (exit, minimum depth);
  - a note.

  It also maps every spot04 surface type to a collision role, and carries the measurements of the
  generated pieces. `overworld kit-pieces` (`overworld::pieces`) cuts the pieces out of the extract
  into `out/overworld/kit/kokiri/pieces.json`. It reads `spot04.glb` for the meshes (normals, UVs,
  each material's tint) and `collision.json` for the collision.
  - A box takes every connected piece of a room mesh lying wholly inside it; with `materials`, it
    takes the faces of those roles instead (the pieces joined to the terrain).
  - Collision is every poly wholly inside the box. A surface type the manifest doesn't map fails
    the cut.
  - A door's exit index is checked against the doorway's floor. The counts match the old Blender
    kit's for the same boxes (tested).
  - The editor cuts the kit on first start, and again when the manifest is newer.
- **A piece's frame:** origin at its base, +y the way it faces. A house's origin is its doorway's
  floor; Link's house's is its foot, with the door 181 up on the porch. A stone's origin is its top.
  An opening's origin is the foot of its mouth, on the wall's face.
- **Scale limits per piece.** Houses scale ×1 to ×1.5, uniformly: a door may get bigger, never
  smaller. Link's house, whose ladder and porch fit child Link, and the log tunnel's mouth are
  locked. The hedge's height is locked: 28 is a climb.
- **Props in the document:** `props: [{ piece, at, z?, yaw, scale }]` (`overworld::props`). After the
  painted terrain moves the ground, and before lighting:
  - each prop stands on the highest floor under its anchor: a house's door at its base, otherwise
    its origin;
  - stones go in water, with their tops 15 above the surface, as spot04's are;
  - a prop with `z` stands at that height instead.

  A prop's triangles keep their own normals and tints (object `props`). Its collision becomes
  never-drawn triangles (object `props_collision`, `"render": false`, material -1). Uneven ground
  under a footprint (more than 20 below the base) is reported.
- **Kokiri's ground in the library** (`kit::ground_with_detail`): `kf_ground` is the mix baked into
  one 256 x 256 tile, the camo filtered up with the detail repeating 8 x 8 across it. Floors keep
  one texture per 400 units and look as the game's do. The extractor now records tile 1's texture
  (`n64_texture1`); for older extractions it's the texture added right after tile 0's.
- **Camera-only collision** (polys flagged to ignore entities: the hedge's walls and top) is left
  out of the kit, so Link wades through the tall grass on its own floor, as in the game.
- **Collision roles** grow in `oot_import::level::ROLES`, with spot04's words (bg camera cleared):
  - dirt, stone, planks, wood, tall grass and fence (the hookshot holds);
  - ladder and ladder top (wall types 2 and 3), crawl (wall type 5);
  - door and exit: plain floor, since an exit in the spikes' view stops on a black screen. They
    lead somewhere once levels load through `Play_Init` (ADR 0035's next step).
- **Decals.** The texture library records `n64_decal`. The export marks those materials
  `"decal": true`. The game draws them in the decal depth mode, and the editor's 3D view with a
  depth bias. Door shadows come with their houses that way.
- **The collision budget.** The builder counts the collision vertices as `CollisionBuilder` will
  (corners merged at whole units, water left out): `Level::collision_vertices`. The editor shows
  that count against 8192. The village example's count (2,933) equals the game's own.
- **The pieces generated along lines** follow the survey and the user's choices:
  - **The dirt path** is cut into the floor web and drawn with a two-texture blend: grass and dirt
    share world-projected UVs, blended by a per-vertex weight (Kokiri's own ground combiner, with
    shade alpha for env alpha). It is not a decal on top. Built:
    - `Doc::lines` of kind `dirt`, the theme's `dirt` section;
    - the floors take points on two rings, with constraint edges between them;
    - the material `ground+dirt` and its composite second texture (`textures::composite_name`);
    - `"alpha"` per object in `level.json`;
    - `oot_import` and the editor's 3D view draw two textures.
  - **Fences** (lines `fence`, `lattice`): whole texture repeats per stretch, so a post stands at
    every node. Their feet follow the ground, with points only where it bends. They collide from both
    sides.
  - **Hanging bridges** (line `bridge`): a cut-out plank deck, and ropes from the fence texture's top
    rail (`kf_fence@1-8`, a derived rows texture: spot04 has no rope), with posts at the anchors.
    - Each span sags on a catenary, 6% of the span; the ends land at their floors' edges.
    - The deck collides as planks, with invisible walls along its edges.
    - Ends steeper than the walkable slope are reported.
  - **Openings and wall pieces** (`src/openings.rs`) fit themselves to the nearest wall, facing out.
    - An opening needs height and room behind; a log's cone is cut back to the room, and its wall
      may curve (its rim stands out from it).
    - A crawlspace is stretched through a ridge to the floor beyond. Both walls must be flat (within
      4) and parallel (within 4°), and the floors at both ends level, so both arches sit flush.
    - Its floor calls for a `CAM_SET_CRAWLSPACE` bg camera, its line as spot04's. The level's bg
      cameras are exported (`cameras`, roles `<role>#k`), with `NORMAL0` first for every other
      floor. A custom level's camera starts as `Play_Init`'s does (`PlayState::play_init_camera`),
      so floors' bg cameras apply and `Camera_Subj4` carries Link through, as in the game.
    - Wall pieces need a flat wall (within 3), reach its top, and collide where they're drawn.
    - The wall's triangles round the mouth are replaced by a triangulation of their area less the
      mouth's outline, each new point on the old surface with its UVs, material and collision.
      The wall stays closed everywhere but the mouth. The mouth's own points lie on the piece's
      front, so a curved wall bends to meet the rim.

## Consequences

- `sketch_village` (made by `make_levels.py`) has everything. Its parts:
  - six houses, stumps, a hedge and stepping stones;
  - dirt paths between the doors, a fenced pen and a lattice;
  - a lookout joined to a plateau by a rope bridge, with a crawlspace through it and vines up its side;
  - a log tunnel in the edge of the world.

  It's 5,793 triangles, and its 2,933 collision vertices are what the game counts. In the game:
  - Link climbs the ladder of Link's house onto its porch;
  - the stones stand in the pond;
  - Link walks into a fitted log tunnel;
  - he crawls through a crawlspace set into the lookout (`--script crawl`);
  - he crosses the rope bridge;
  - he stops at a fence.
- Not yet:
  - doors and exits that lead somewhere (`Play_Init`, ADR 0035's next step);
  - flattening the ground under a prop;
  - the waterfall's scrolling texture.
- The kit is ROM data and stays in out/. Only the manifest (boxes, origins, limits, surface roles)
  is committed.
