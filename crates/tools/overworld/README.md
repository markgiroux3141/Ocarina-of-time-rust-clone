# Overworld level builder

The geometry core of a future standalone overworld level editor: Kokiri Forest-style levels
from an outline, regions raised or sunk inside it, and an edge of the world that builds itself.
Pure Rust with no engine and no Blender. A level document (JSON) and a theme (JSON) go in;
textured, watertight meshes come out. The editor (`../overworld_editor`) draws and edits documents, and
the game plays the export with child Link (`oot_sandbox --level`, `oot_import::level`, ADR 0035).

This crate started in the Blender MCP for level generation repo (pd-walk walking, Blender previews) and
moved here on 2026-10-05; its history up to the move is there (commit `21216d9`).

![The sketch level: floors by height, the bank shaded by the rim height, the tree line in green](docs/sketch_plateau_plan.png)

Builds and their textures (ROM data, so never committed) go in `out/overworld/`.

## Status (first milestone, 2026-10-04)

Done:
- An outline with **regions** inside it: floors raised or sunk, and ponds. Regions can be free-standing
  or drawn against the outline (sharing its nodes).
- **Walls** wherever heights differ, styled and textured automatically.
- **The boundary:** a cliff from each floor up to the rim, a bank, then trunks and foliage standing on
  the bank's edge, attached. The rim **rises gradually** over high ground near the edge.
- `tools/trace_sketch.py` turns a Paint sketch into a document. The test levels in `examples/sketch`
  are traced from the user's drawing at 8 units per pixel: about 7,750 x 5,350, roughly 2.8 times
  Kokiri village's width. The user wants big maps, where the same walls are short against the level.

- **Paths** (2026-10-05): ramps and embankments joined to the ground, bridge decks with open space under them, and
  paths that switch from one to the other. All walked end to end in pd-walk before the move (`examples/sketch/sketch_paths.json`).
  Since then, a ramp running on past a plateau's edge cuts into the plateau instead of landing at the edge.

- **The editor** (2026-10-05, `../overworld_editor`, see its README): draw and edit the outline, regions and paths over a
  plan of the built level, with a 3D view beside it and a side profile for paths, where heights are set by dragging. It
  rebuilds in the background on every edit, and **▶ Play** plays the level in the game with child Link and the pad, which
  reloads each rebuild.

- **Bumps** (2026-10-05): smooth noise on the ground and on any region's floor, fading out towards each floor's edges.
  Walls, the rim, paths and bridges are untouched (tested). Example: `examples/sketch/sketch_bumpy.json`.

- **Painted terrain** (2026-10-05, `src/terrain.rs`): a smooth height offset over the whole level, painted with the
  editor's brush. Everything rides on it; ponds stay level. Example: `examples/sketch/sketch_hills.json`.

- **Detail** (2026-10-05): `settings.detail` high (the default, unchanged), medium or low. `sketch_hills` is 24,254 /
  7,815 / 5,011 triangles and looks nearly the same at each.

- **Props** (2026-10-05, ADR 0036): Kokiri Forest's houses, stumps, stepping stones, hedge, log tunnel and crawlspace,
  cut from the extract by a committed manifest (`kit/kokiri.json`, `src/pieces.rs`) and placed in the document (`props`,
  `src/props.rs`). They stand on the finished ground, with their collision, and are counted against the game's 8192
  collision vertices. Example: `examples/sketch/sketch_village.json`.

- **Dirt paths** (2026-10-05, `lines` of kind `dirt`, `src/lines.rs`): painted into the floor, not laid on it. The floors
  take points on rings round the path, and the ground blends from grass to dirt by vertex weight, as Kokiri's ground
  combiner blends its two textures.

- **Fences and hanging bridges** (2026-10-05, `lines` of kind `fence`, `lattice`, `bridge`): fences stand on the ground with
  a post at every node; rope bridges sag between their anchors on a catenary, and Link walks across.
- **Hedges** (2026-10-05, `lines` of kind `hedge`): walk-through tall grass over any shape of nodes, drawn like a region.
- **Wall openings** (2026-10-05, `src/openings.rs`): the log tunnel and the crawlspace set into the wall nearest where they're
  put, with a gap cut to fit; vines and the waterfall stand on walls. Link walks into the log and crawls through the
  crawlspace.

Not yet: exits and doors that lead somewhere (they wait for levels to load through `Play_Init`).
See the roadmap.

## Usage

From the repo root:

```
cargo run --release -p overworld_editor -- crates/tools/overworld/examples/sketch/sketch_paths.json
cargo test --release -p overworld -p overworld_editor
target/release/overworld kit-textures       # extracted/scenes/overworld/spot04/spot04.glb -> out/overworld/textures/kokiri
target/release/overworld kit-pieces         # kit/kokiri.json + the extract -> out/overworld/kit/kokiri/pieces.json
target/release/overworld build crates/tools/overworld/examples/sketch/sketch_plateau.json out/overworld/sketch_plateau --textures out/overworld/textures/kokiri [--kit out/overworld/kit/kokiri]
target/release/oot_sandbox --level out/overworld/sketch_plateau --child   # play it (reloads on rebuild; --at x,y,z,yaw places Link)
python crates/tools/overworld/tools/trace_sketch.py sketch.png level.json --scale 8 --region blue:z=120 --region red:kind=water,z=-100,surface=-20
python crates/tools/overworld/tools/plan.py <out>/level.json plan.png --doc level.json   # floors by height, bank, walls, tree line
```

`OW_TIMING=1` prints each build phase's time (the CLI and the editor; the editor adds its own handling of each build).
`sketch_hills` builds in about 75 ms: the tree line's distance field and the floors' edge distances use a segment index
(`geom::SegIndex`, `row_crossings`) instead of testing every outline segment, which took the edge of the world from
about 200 ms to 12, with byte-identical output.

Output (`<out>/`): `level.json` holds objects (ground, water, walls, boundary, trees, foliage, overlays)
with vertices, triangles, per-corner UVs, a material role and a collision surface role per triangle.
`level.obj` and `level.mtl` are for any tool (OBJ is y-up), with the textures the level uses copied
into `textures/` from the `--textures` library. That's a folder of PNGs plus `textures.json`, giving
each texture's wrap per axis, alpha (opaque, cutout or blend) and culling. For Kokiri it's
`out/overworld/textures/kokiri`, written by `overworld kit-textures` (`src/kit.rs`) from the extracted
scene's glb (ROM data, git-ignored; the editor makes it on first start). The MTL gives `map_Kd`, `map_d`
for alpha and `d` for translucency; material names carry the GE64 dialect's flags
(`forest_trunks_ClampT_Cutout`), which pd-walk and Blender's OBJ import read. In `level.json`, the axes
are x east, y north, z up, and heights are absolute: the game reads it (`oot_import::level`, ADR 0035).

## The level document

```json
{ "name": "sketch",
  "outline": { "nodes": [[x, y], ...], "z": 0, "noise": { "amplitude": 70, "scale": 900, "edge": 300 } },
  "regions": [ { "name": "pond", "nodes": [[x, y], [x, y, 1], ...], "z": -100, "kind": "water", "surface": -20 },
               { "name": "ledge", "nodes": [...], "z": 160, "edge": "vines", "noise": { "amplitude": 30 } } ],
  "paths": [ { "name": "ramp", "nodes": [[x, y], [x, y]] },
             { "name": "bridge", "nodes": [[x, y], [x, y, z, width], [x, y]], "mode": "floating" },
             { "name": "climb", "nodes": [...], "modes": ["attached", "floating"], "width": 160, "edge": "vines" } ],
  "boundary": { "cliff_min": 280, "bank": 220, "bank_rise": 80, "rise_slope": 0.25, "reach": 300, "panel_tol": 90 },
  "settings": { "sample": 60, "steiner": 250, "weld": 1 },
  "props": [ { "piece": "saria_house", "at": [200, -1300], "yaw": 20 },
             { "piece": "stone_large", "at": [-420, 860] },
             { "piece": "hedge", "at": [-1700, 600], "yaw": 30, "scale": [1.5, 1.5, 1] } ],
  "lines": [ { "name": "main path", "kind": "dirt", "nodes": [[-1470, -1180], [-900, -800], [-250, -650]], "width": 160 },
             { "name": "pen", "kind": "fence", "nodes": [[-40, -230], [-420, -260], [-440, -520]], "closed": false },
             { "name": "rope bridge", "kind": "bridge", "nodes": [[880, 1330], [1780, 1760]] } ] }
```

- **Nodes are control points.** Edges between them are smooth curves (centripetal Catmull-Rom). A
  third value of 1 marks a sharp node. `settings.edges` changes that for the whole level (see Edges below).
- **One web of shared nodes.** A region's node within `weld` of another loop's node *is* that node,
  so a region drawn against the outline shares the outline's edge. Each edge is sampled once, and
  everything touching it uses the same points, which is why the result is watertight. Loops may share
  nodes and edges but may not cross or overlap (refused with the place where they do).
- **Faces.** The web cuts the plane into faces. Each face belongs to the innermost loop around it,
  which gives its height, so nesting (a region inside a region) just works.

## Bumps (`noise` on the outline or a region)

`{ "amplitude": 20, "scale": 600, "edge": 200, "seed": 0 }` (the defaults): smooth noise (`noise::relief`, three octaves,
stretched to use its range) of up to `amplitude` up or down, with features about `scale` across. It fades to nothing
within `edge` of each face's edges, so every edge keeps its floor's height. That's what keeps the rest of the level exactly
as it was: wall tops and feet, the rim, path landings and bridge ends all sit on edges, and faces under an attached path
take the path's surface with no bumps (tests `bumps_change_only_the_ground_inside_floors`,
`paths_keep_their_surface_over_bumpy_ground`). Noisy floors get interior points every `scale / 4` (at least 40, at most
`steiner`), so `sketch_bumpy` has about 650 more ground triangles than `sketch_paths`. A pond's bed stays 5 under its
surface. Each region's pattern differs; `seed` and `settings.seed` vary them.

Seen from PD's eye height, ±35 hardly shows, because the theme's shade variation mottles flat ground as much. ±70 over
900-unit hills reads well. The edge fade hides bumps where
they'd show most, at the foot of walls. Letting them run to walls (walls following them) is the next step, with the brush.

## Painted terrain (`terrain` in the document, `src/terrain.rs`)

`{ "cell": 50, "detail": 100, "chunks": { "cx,cy": [256 offsets], ... } }`: height offsets at grid nodes `cell` apart,
in 16 x 16 chunks (missing chunks are flat), bilinear between nodes. The editor's brush paints it: raise, lower, smooth
(towards the neighbours' average, over a kernel that grows with the brush), flatten (towards the offset where the stroke
started), bumps (`noise::relief`) and erase.

It's applied to the **finished** level as a deformation: every vertex moves up by the offset at its (x, y), before the
lighting is baked (so hills are shaded). Because the offset depends only on position, shared vertices stay shared and
vertical walls stay vertical: walls keep their heights, ramps and bridges keep their shapes relative to the ground, and a
hill painted under a plateau lifts the plateau, its ramps and the bridge ends landing on it. Region heights stay what they
are in the document. Water has to stay level, so a pond takes the offset at its middle all over, blending back to the
painted offset within 250 of its shore (`terrain::Field`). Floors get extra points every `detail` wherever the terrain
varies. A path the terrain steepens past walkable is reported. Tests: `painted_terrain_moves_everything_with_the_ground`
(every non-ground vertex moves by exactly the offset under it, still watertight, ponds level) and the brush tests in
`terrain.rs`. `sketch_hills` (a hill under the island, a rise and a hollow, written by `make_levels.py`) was walked end to end in
pd-walk before the move, with one waypoint on the hill's flank as a pass-through.

## Edges (`settings.edges`)

How the outline's, regions' and paths' edges run between their nodes (2026-10-05; dirt lines follow the paths'):

| | |
|---|---|
| `smooth` (default) | The curves, as before. |
| `faceted` | The same curves as a few long flat panels: corners where the curve strays more than 20 (`doc::FACET_TOL`, or the detail's tolerance if larger) from a straight piece, about 16 round a circle 1000 across. Low-poly, as the game's walls are. |
| `hard` | Straight from node to node, every node a corner: the shape is exactly the nodes you placed. |

The straight pieces still get a point every `sample` (or the detail's longest piece; `geom::Sampling::Facets` and
`Straight` cut along the chord), so painted terrain and walls follow the ground along them. That's why faceted
isn't lighter at high detail, but at medium `sketch_village` goes from 5,737 triangles to 3,982 (faceted) and 3,796
(hard). Straightened edges can cross where the curves didn't: the build reports where. Test:
`faceted_and_hard_edges_are_lighter_and_still_watertight`.

## Wall texture (`settings.wall_texture`)

How capped walls (the cliffs, the edge of the world's too) are textured (2026-10-05):

- `tiled` (the default): the grassy top and bottom caps keep their size and the rock between repeats, so every wall is
  as sharp as a short one (see Automatic texturing).
- `stretched_middle`: the caps keep their size and the rock between is stretched once over the rest of the wall. The
  middle is plain rock, so it blurs little, and the crisp grass edges are what make walls look sharp. Lighter than tiled.
- `stretched`: the texture once over the wall's height (v 0 at its foot, 1 at its top, the caps stretched with it), and
  across it as many units per repeat as keeps the texture's shape (`tile_u` x the run's mean height / `tile_v`). This is
  how Kokiri Forest's own walls are: blurrier on tall walls, and where walls of different heights meet the texture
  doesn't line up. Embankment sides and rock arches stay top-anchored. Test: `walls_tile_or_stretch`.

`sketch_village`, triangles at high / medium / low: tiled 11,075 / 5,589 / 3,960, middle stretched 9,043 / 5,589 /
3,960 (the same at medium and low: one middle band either way), stretched 7,007 / 4,145 / 3,028. Before the middle grew
(below), tiled was 13,467 / 5,737 / 4,076.

## Detail (`settings.detail`, `doc::Detail`)

| | High (default) | Medium | Low |
|---|---|---|---|
| Outline and region curves | every `sample` (60) | within 2.5 of the curve, pieces up to 4 x `sample` | within 6, up to 8 x `sample` |
| Path stations (embankment sides, bridge arches) | every `sample / 2` | within 2.5, up to 2 x `sample` | within 5, up to 4 x `sample` |
| Capped walls | a band per middle repeat | three bands | three bands |
| Flat floors' points | `steiner` | 1.6 x | 2.4 x |
| Painted terrain's points / bumps' points | `detail` / scale over 4 | 1.5 x / over 3 | 2 x / over 2 |
| `sketch_hills` | 24,254 triangles | 7,815 | 5,011 |

High builds exactly what it always did. Adaptive curves (`geom::sample_curve`) sample densely, keep what Douglas-Peucker
needs to stay within the tolerance and cut long pieces evenly, so straight stretches become long panels, as Kokiri's
are. **Three-band walls** are the big saving: the cliff texture's middle used to be a band of its own for every
repeat (about 13 bands on a 400-unit cliff, when the middle was rows 5 to 9). At medium and low the middle is one band
with a texture of its own, the middle rows, mirror-repeating vertically. That texture is derived from the library's PNG
(`textures::derived_name`, e.g. `kf_cliff@5-26m`, written as `kf_cliff-rows5-26.png`, material role `cliff~mid`, OBJ
material `cliff-mid_MirrorT`), so the kit needs nothing new. Three-band walls don't fold (see Automatic texturing): the
fold's two bands cost about a fifth more, so their repeats stretch a little instead. Tests: `lower_detail_is_lighter_and_still_watertight`,
`derived_textures_are_rows_of_their_base`. Before the move, Low walked pd-walk's route over `sketch_hills` with
the two hill-flank waypoints as pass-throughs. For N64 use the derived middle (32 x 5) isn't a power of two yet.

For comparison, Kokiri Forest is about 3,750 triangles in all, in about an eighth of the sketch levels' area. A real
N64 port would also need the level split into rooms.

## Paths (`src/paths.rs`)

A path is a line of nodes `[x, y]`, `[x, y, z]` or `[x, y, z, width]`. Between nodes it's a smooth curve, with
heights linear along it.
- **Heights fill themselves in.** An end with no z takes the floor's height there, and a node between ends with
  none is interpolated. A ramp is just two points, one on the ground and one on a plateau.
- **Cuttings.** An attached end's slope runs all the way to its node. A ramp from the ground to a point halfway
  into a plateau rises to the plateau's edge as an embankment, then goes on into the plateau as a cutting, reaching
  the top at the node. Put the top node at the edge to land flush there (a node a little inside leaves a low lip).
- **Landing.** A floating end standing on a floor of its own height, where the ground falls away further in, lands
  at that floor's edge: the deck starts exactly at the edge, however far onto the plateau the node is.
- **Attached** segments are embankments and cuttings. Their footprint joins the web, and inside it the ground is the
  path's surface, above the region's or below it (where two paths overlap, the higher). A ramp drawn overlapping a
  cliff therefore meets it edge to edge if it's higher there, and cuts a notch into it if it's lower. The forest rim
  rises over a path like any high ground. Footprints may cross anything: every crossing becomes a vertex, and the
  part outside the outline is cut off. Where a ramp rises past the floor beside it, the wall between them changes
  sides at the crossing point. The sides, an embankment's and a cutting's, use the theme's `embankment` style (a
  path's `edge` overrides it). That style is **top-anchored**: the grass lip follows the top edge, and the rock
  repeats down at its own size until the ground cuts it off. Test: `a_ramp_into_a_plateau_cuts_in`.
- **Floating** segments are bridges. The deck's top is world-projected. Where it lands, its end follows the floor's
  own edge: the deck's sides are passed to the map as probes, so their crossings become floor vertices, and the end
  shares them. Below the deck is the theme's (or the path's `shape`) body:
  - `rock`, a natural arch (Zora's River style). Each cross-section runs down a lip from the deck's edge, bulges
    out, and curves round to a rounded bottom. It's deep where it meets the ground and thin mid-span, with lumpy
    noise. It's textured like a top-anchored wall, by arc length down from the deck's edge, so the grass lip sits
    under the deck's edge. Both halves meet on the bottom line, vertex for vertex. It carries on into the ground it
    lands on, hidden behind the cliff, and back into an embankment it continues, without bulging out of it.
  - `slab`: an underside and edge strips over the deck's `thickness`.

  There is an end face only where an end is free.
- Slopes steeper than the theme's `max_slope` (35°) are reported. A footprint that folds over itself (too tight
  a turn for its width) is refused.

**Stitching.** Everything meets vertex to vertex:
- Floors take extra points on their edges wherever an embankment's texture bands meet them, or a wall changes
  sides partway along.
- Top-anchored walls split at every height another wall meets their ends.
- Deck ends share the floor's edge.

Tests: `levels_with_paths_are_watertight_too` (ground, walls, cliffs, bank and trees share every edge) and
`a_deck_meets_the_floor_it_lands_on_edge_for_edge`.

Not yet: railings, and supports under long bridges.

## Props and the kit (`kit/kokiri.json`, `src/pieces.rs`, `src/props.rs`, ADR 0036)

**The kit.** `overworld kit-pieces` cuts Kokiri Forest's pieces out of the clone's extract (`spot04.glb` for the meshes,
`collision.json` for the collision) into `out/overworld/kit/kokiri/pieces.json` (ROM data, never committed). The editor
does it on first start and whenever the manifest changes. The manifest says, per piece:

- **Source:** a box in the scene (x east, y north, z up). It takes every connected piece of a room mesh wholly inside it,
  or, with `materials`, the faces of those roles (for pieces joined to the terrain: the log tunnel, the crawlspace).
  Collision is every poly wholly inside the box. Each spot04 surface type maps to a collision role in the manifest's
  `surfaces`; an unmapped one fails the cut.
- **Frame:** `origin` (base, top, door, or a point) and `facing` (door, normal, or a direction). A piece is stored with
  its origin at 0 and facing +y.
- **Scale limits**, **door** (exit and entrance, checked against the doorway's floor) and **opening** (for pieces set
  into walls).

| Piece | Size | Notes |
|---|---|---|
| `link_house` | 554 x 587 x 435 | Door on the porch, 181 up; the ladder climbs to it. Fixed size. |
| `mido_house`, `saria_house`, `twins_house`, `knowitall_house`, `shop` | 185 to 349 tall | Doorway at the origin. x1 to x1.5. |
| `stump_post`, `stump_post_tall` | 98 x 98 x 120 / 180 | Where spot04's plank walkways land. |
| `stone_small`, `_medium`, `_large` | 80 to 120 wide | Origin on top: stands in water 15 above the surface. |
| `hedge` | 174 x 235 x 28 | Tall grass: Link wades through it (its walls and top only stopped the camera, so the kit drops them), on tall-grass footsteps. Spot04's has skirts on two sides; the kit adds the rest (`close_sides`). Kept for older levels: new hedges are drawn as shapes (Hedges below), and the editor's Kit panel no longer lists it. |
| `log_tunnel` | mouth 221 x 220, 857 deep | Both of spot04's log exits (Hyrule Field, Lost Woods) are this model. |
| `crawlspace` | 40 wide, 27 high, 320 long | Wall type 5 at both mouths. |
| `vines`, `waterfall` | | Wall pieces: climbable vines, a translucent fall. The vines tile (`"tiles": true`): see Wall openings. |

Houses bring their door shadows (decals) and Link's house its graffiti and mushrooms.

**Props.** `props: [{ "piece", "at": [x, y], "z"?, "yaw"?, "scale"? }]` in the document. `yaw` is degrees
counter-clockwise from north; `scale` is along the piece's own axes, kept within its limits. After the painted terrain
moves the ground, and before lighting:
- each prop's origin stands on the highest floor under its anchor (a house's doorway, otherwise its origin), or at `z`
  if it has one;
- a stone stands in water 15 above the surface.

Its triangles keep the source's normals and tints (object `props`). Its collision goes into object `props_collision`,
never drawn (`"render": false`), with roles the game knows: wood, dirt, stone, planks, fence, ladder, ladder top, crawl,
door, exit. Doors and exits collide as plain floor: they lead somewhere once levels load through `Play_Init` (ADR 0035's
next step). Ground falling more than 20 below a prop's base under its footprint is reported.

**The collision budget.** `Level::collision_vertices` counts what the game's collision will hold (corners merged at
whole units, as `CollisionBuilder` does): at most 8192. `sketch_village` at medium detail is 2,933, the game's own count.

Tests: `the_kokiri_pieces_come_out_of_the_extracted_scene` (the counts match the old Blender kit's; doors, the ladder,
the log tunnel's opening, the crawlspace; skipped without the extract), `props_stand_on_the_ground_turned_and_scaled`,
`houses_stand_on_their_doorway_and_stones_in_water`. In the game, Link climbs Link's house's ladder onto its porch
(`oot_sandbox --level out/overworld/sketch_village --child --at=-1470,250,1000,0 --script hold --frames 300`).

Openings and wall pieces (kinds `opening` and `wall`: the log tunnel, the crawlspace, the vine patch, the waterfall) aren't
stood on the ground but fitted to the nearest wall: see Wall openings.

## Dirt paths (`lines` of kind `dirt`, `src/lines.rs`, the theme's `dirt`)

Kokiri Forest's yellow paths are 22 decal quads: strips, junctions and end caps, whose textures are white blotches with
soft alpha, tinted yellow-brown at 70%. Decals would fight the floor for depth, so here a dirt path is painted into the
floor:

- **Cut into the floor.** A line of nodes is a smooth curve (as paths are), `width` across (the theme's 160). Two rings run
  round it, round both ends: one where the dirt is full (half the width less half the soft edge) and one where it ends.
  The floors take points on both rings, with constraint edges between them, so the soft edge is a band of the floor's own
  triangles. Each floor vertex gets a weight (1 to 0) from its distance to the centre line, and the edge wanders by up to
  `wobble` (smooth noise along the line).
- **Blended in.** The floor's triangles that touch dirt take the material `ground+dirt`, which draws two textures, blended
  by the vertices' weights. In the game it's Kokiri's own ground combiner, (TEXEL1 - TEXEL0) x weight + TEXEL0 then x
  shade, with the weight in the vertex alpha (`alpha` in `level.json`). Texture 1 is texture 0 with the dirt drawn over it
  (`textures::composite_name`): the decal strip's middle columns, mirrored across so it tiles, tinted and at its opacity,
  twice per floor tile, at four times the floor texture's size. Both textures share the floor's world-projected UVs.
- **Dirt underfoot.** Floor that's mostly dirt collides as `dirt` (spot04's surface 13, dirt footsteps).

Paths may cross and run up ramps. Where two overlap, the stronger weight wins, and a ring point that lands inside
another stretch of dirt is left out, so its constraint edge isn't added. Test: `dirt_paths_are_cut_into_the_floor` (the
level stays watertight, there are points on both rings, the materials and surfaces). The editor's 3D view draws the
blend, and the plan tints the floor by the weights.

## Fences (`lines` of kind `fence` or `lattice`, the theme's `fences`)

Spot04's fences are 40-tall quads with a post drawn into the texture every 40; the lattice by the crawlspace is 120 tall,
30 per repeat. A fence line is straight between its nodes (`closed` joins the last to the first). Each stretch gets a
whole number of repeats, so a post stands at every node. Its foot follows the finished ground: points every repeat,
kept only where the ground bends more than 3. The panels are drawn once (the texture is double-sided) in object
`fences`, and collide from both sides (`fences_collision`, as `fence`, which the hookshot holds, or `wall_nograb` for the
lattice). Points off the ground are reported. Test: `fences_stand_on_the_ground_with_a_post_at_every_node`.

## Hanging bridges (`lines` of kind `bridge`, the theme's `hanging`)

Spot04 has rigid plank walkways but no rope bridge, so this one is made in its textures:

- **Deck:** planks (`kf_log_side` on top, `kf_log_end` beneath, both cut-out, so there are gaps between the planks), 80
  wide, a plank every 20.
- **Ropes:** strips of the fence's top rail (`kf_fence@1-8`, rows 1 to 8). A hand rope runs 50 above each edge, with
  uprights every 60. A post (`kf_post_bark`) stands at each corner.

Each span between nodes sags on a catenary, 6% of its span in the middle. Each anchor stands on the ground there, or at
its node's third value. The two ends **land at their floors' edges**: put them anywhere on the floor they start from, and
each moves towards the other anchor while the floor stays at its height, as paths' ends do. A deck end steeper than the
walkable 35° is reported.

The deck collides as `planks` (bridge footsteps), with invisible `wall_nograb` walls 70 tall along both edges so Link
stays on. In `sketch_village`, Link walks from the lookout across to the plateau, dipping 35 in the middle. Test:
`hanging_bridges_sag_by_their_span_and_report_steep_ends`.

## Hedges (`lines` of kind `hedge`, `lines::hedge`, the theme's `hedge`)

Kokiri's tall grass over the closed shape of a line's nodes (always closed, straight between nodes; a shape crossing
itself is reported). Its top stands `height` (28) above the ground everywhere, with points every `spacing` (100) inside
and along the edge so it follows hills, textured `hedge_top` world-projected every 80 (spot04's density). Grass skirts
(`grass_skirt`, every 140 along, as the kit's) face out round it, down to the ground. Neither collides: under it, a
floor of tall-grass footsteps 2 above the ground does (object `hedges_collision`), so Link wades through, as through
spot04's. Built after props, on the finished ground. Test: `hedges_cover_their_shape`.

## Wall openings (`src/openings.rs`)

A prop whose piece is an opening (`log_tunnel`, `crawlspace`) or a wall piece (`vines`, `waterfall`) is fitted to the wall
nearest its `at` (within 300): its origin goes on the wall's face, at the floor in front, facing out over it. Its `yaw`
and `z` are ignored.

It has to sit on one face: the wall from corner to corner (a turn of more than 60 degrees, as at a hard or faceted
edge's node) must be at least its width plus 20 each side. Put down near a corner, it slides along the wall until it's
clear of it; on a face too narrow it's refused ("the wall is 201 wide here (corner to corner); it needs 262"). Gentler
turns count as the same wall, as far as they stay within 60 of its plane, even where a long face (low detail) strays
further beyond the mouth (`openings::span`; the cut's `same_wall` likewise only looks across the mouth).

- **Room.** An opening needs:
  - a wall tall enough for its mouth, plus 10;
  - space behind: ground above the mouth for at least its `min_depth`, or the edge of the world.
- **The log tunnel** stands well out from the wall, so its wall may curve up to 40 off flat across the mouth. Its cone is
  cut back (y scale, down to 0.4) to the room there is.
- **The crawlspace** goes through a ridge to the floor beyond. It's stretched to the ridge's depth (its y scale, 0.5 to 3),
  and the far wall gets a gap too. Its arches lie flat on the walls, so:
  - both walls must be flat (within 4 across the mouth) and parallel (within 4°);
  - the floors at both ends must be level (within 6).

  Draw the ridge with sharp corners (straight, parallel edges). Its floor calls for a crawlspace camera
  (`CAM_SET_CRAWLSPACE`, the line through it, 22 past each mouth and 12 up, as spot04's), so `Camera_Subj4` takes over
  while Link crawls, as in the game (`Mesh::cameras`, `level.json`'s `cameras`, the role `crawl_floor#k`).
- **Wall pieces** (vines, the waterfall) lie flat on the wall from its foot to its top (z scale to fit). They need a flat
  face (within 3 of a plane, `WALL_PIECE_FLAT`) as wide as they are plus 10 each side; near a corner they slide along
  it like an opening, and on a face too narrow the problem says how wide it is (`Fit::room`). The x scale is their width.
  The vines' collision lies where they're drawn, just in front of the wall, so Link climbs them anywhere.
- **Tiling** (a piece's `tiles`, set for the vines): its texture repeats at the piece's own density as it's scaled
  (`props::tiled_uvs` evaluates each triangle's own texture mapping at the scaled corners), instead of stretching, so a
  patch can be any width and reach any height: the height limits don't apply. Test: `vines_tile_and_reach_the_top`.
- **Previews.** `openings::preview` fits a piece where it would be put down without building: its triangles, the
  fitted prop and the face's width. The editor draws it as a ghost.

If a piece doesn't fit, the problem says why: too low, not flat, not parallel, no room, no floor beyond.
- **The gap.** The wall's triangles round the mouth come out, grown until the mouth is inside them. The area they
  covered, less the mouth's outline (the piece's front seen face on, kept a unit above the floor), is triangulated again.
  Each new point lies on the old wall's own surface and takes its UVs from the old triangle under it. So the texture
  carries on, the material and collision stay the wall's, and the wall is closed everywhere but the mouth. The mouth's
  own points lie on the piece's front, so a curved wall bends to meet the log's rim with no gap. Test:
  `openings_are_set_into_walls` (also refuses a crawlspace through a round island).

The log tunnel's exit floor and the crawlspace's wall type 5 come with their collision. In the game (`sketch_village`),
Link walks 740 into the log. With A at the crawlspace's mouth (`--script crawl`) he crawls through to the far side,
seen from the crawlspace camera. He climbs the vines anywhere across them onto the lookout.
The log's exit leads nowhere until levels load through `Play_Init`.

## Automatic texturing (`themes/kokiri.json`)

The document never names a texture. The builder classifies every piece of geometry and the theme maps
each class to a material, a tiling and overlays:

| Geometry | Rule | Kokiri |
|---|---|---|
| Floors, the bank | World-projected UVs: seamless across faces, regions and the bank | `ground`, 400 per tile: Kokiri's two textures as the game mixes them, the camo with a detail texture eight times finer (`kit::ground_with_detail`, 256 x 256) |
| Pond beds, water | World-projected | `ground`; `water` at 54 per tile |
| Walls | First matching `wall_rules` entry: over water, then height | Shore `cliff_strip_dark`; up to 75 high `cliff_strip` (ledge); taller `cliff` |
| | A region's `edge` overrides the rules for its own walls | e.g. `vines` |
| Wall U | Arc length round the face, continuous round corners, **snapped to whole repeats round a closed loop**, so there's no seam | |
| Wall V | **Caps:** the texture's grassy top and bottom keep their own size on every wall, and a middle section repeats as many times as the height needs, so tall walls are as sharp as short ones. Walls no taller than the texture show it once. The middle runs from texel centre to texel centre and can be **mirrored** (every other repeat upside down), so repeats meet texel for texel; mirrored, it repeats a whole odd number of times and **folds** the rest (a part-repeat up into the middle and back down at its foot), so every texel keeps its size on every wall (at high detail; three-band walls stretch the repeats instead, at most about 1.7 x). The repeat count is fixed per wall run, so neighbouring segments match. `settings.wall_texture` can stretch the middle or the whole texture instead | `cliff`: rows 0-4 top, rows 26-31 bottom, rows 5-25 (the dark rock under the lip down to the light mossy rock) repeat, mirrored, at 167 units per texture height: one repeat is the texture as drawn. The joins fall on rows 5 and 25, alike either side, so they don't show as lines. The middle used to be rows 5-9, 26 units, about ten thin stripes on a 400-unit wall |
| Overlays | Optional: a skirt along wall feet and a fringe from tops (`skirt`, `fringe`) | Off: the cliff texture has its own grass edges |
| Lighting | Baked into vertex colours, as the N64 lights by normals: ambient + directional lights, normals smoothed per object | Kokiri at 10:00: ambient 80, white key light, dim blue fill |
| Variation | Breaks up repetition with smooth noise, no seams: along walls, u advances at a speed drifting by `u_speed` over about `u_scale` units, so streaks don't land at regular intervals; the baked shade varies by `shade` over about `shade_scale` (not on water). `settings.seed` in the level varies it per level | Speed ±30% over ~450 units, shade ±22% over ~380 |
| Boundary | Cliff (capped like any wall), bank, trunks (480), foliage (460, 180 below the trunks' top, 40 in front) | Measured from spot04 |

Wall columns are split at every height where another wall or floor meets that corner, so walls,
floors and the boundary share every edge (`solid_is_watertight_up_to_the_tree_tops`).

## The edge of the world

The rim (the base of the trees) is `cliff_min + bank_rise` above the outline's floor all round.
Wherever a floor stands on or within `reach` of the edge, the rim at that point is the floor plus
`cliff_min`. The rim is then **slope-limited along the edge** (`rise_slope`, 1 in 4), so it climbs and
falls gradually and never steps (`rim_rises_gradually_over_the_north_plateau`). The tree line is
the outline grown by `bank` (a distance field traced with marching squares), simplified to long
straight panels within `panel_tol`, as Kokiri's dozen or so are. Notches narrower than twice the
bank are bridged by the forest. Trunks stand on the bank's outer edge, so nothing floats.

## Roadmap

1. **Hills and a terrain brush**: done (Bumps, Painted terrain). Next: slopes over about 35° switching to the
   cliff texture automatically, and a steepness overlay in the editor.
2. **Soft edges.** A per-edge style `slope`: a sloped band instead of a vertical wall (Kokiri's north
   rim).
3. **Paths**: done (see above). Next for them: railings and supports.
4. **Checks.** Child Link reachability in the crate: which floors connect, and ledges, vines and swim-outs.
   The game's own movement can now test them too (`oot_sandbox --level` with `--script`/`--trace`).
5. **Props and blocks**: done (ADR 0036): houses, stumps and stones on floors, dirt paths, fences, hanging bridges and
   wall openings.
6. **The editor** (`../overworld_editor`): built, with plan and 3D views, path profiles, the brush and Play.
7. **Play mode** (ADR 0035's consequences): the level as a mod pack so `Play_Init` enters it, actors placed
   in the editor, hookshot targets, exits between levels, and rooms for big levels.
