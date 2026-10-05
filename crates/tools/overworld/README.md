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

Not yet: props and houses.
See the roadmap.

## Usage

From the repo root:

```
cargo run --release -p overworld_editor -- crates/tools/overworld/examples/sketch/sketch_paths.json
cargo test --release -p overworld -p overworld_editor
target/release/overworld kit-textures       # extracted/scenes/overworld/spot04/spot04.glb -> out/overworld/textures/kokiri
target/release/overworld build crates/tools/overworld/examples/sketch/sketch_plateau.json out/overworld/sketch_plateau --textures out/overworld/textures/kokiri
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
  "settings": { "sample": 60, "steiner": 250, "weld": 1 } }
```

- **Nodes are control points.** Edges between them are smooth curves (centripetal Catmull-Rom). A
  third value of 1 marks a sharp node.
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
are. **Three-band walls** are the big saving: the cliff texture's middle (rows 5 to 9, 26 units of plain rock) used to
be a band of its own for every repeat, about 13 bands on a 400-unit cliff. At medium and low the middle is one band
with a texture of its own, the middle rows, mirror-repeating vertically. That texture is derived from the library's PNG
(`textures::derived_name`, e.g. `kf_cliff@5-10m`, written as `kf_cliff-rows5-10.png`, material role `cliff~mid`, OBJ
material `cliff-mid_MirrorT`), so the kit needs nothing new. Each middle texel is 5.2 units, as in the caps (the banded
middle stretched them to 6.5). Tests: `lower_detail_is_lighter_and_still_watertight`,
`derived_textures_are_rows_of_their_base`. Before the move, Low walked pd-walk's route over `sketch_hills` with
the two hill-flank waypoints as pass-throughs. For N64 use the derived middle (32 x 5) isn't a power of two yet.

For comparison, Kokiri Forest is about 3,750 triangles in all, in about an eighth of the sketch levels' area. A real
N64 port would also need the level split into rooms.

## Paths (`src/paths.rs`)

A path is a line of nodes `[x, y]`, `[x, y, z]` or `[x, y, z, width]`. Between nodes it's a smooth curve, with
heights linear along it.
- **Heights fill themselves in.** An end with no z takes the floor's height there, and a node between ends with
  none is interpolated. A ramp is just two points, one on the ground and one on a plateau.
- **Landing.** An end standing on a floor of its own height, where the ground falls away further in, lands at that
  floor's edge. The slope finishes exactly at the edge, however far onto the plateau the node is, and a deck starts
  there.
- **Attached** segments are embankments. Their footprint joins the web, and wherever it overlaps other ground the
  higher surface wins. A ramp drawn overlapping a cliff or the outer wall therefore meets it edge to edge, and the
  forest rim rises over it like any high ground. Footprints may cross anything: every crossing becomes a vertex,
  and the part outside the outline is cut off. Where a ramp rises past the floor beside it, the wall between them
  changes sides at the crossing point. The sides use the theme's `embankment` style (a path's `edge` overrides it).
  That style is **top-anchored**: the grass lip follows the sloping top edge, and the rock repeats down at its own
  size until the ground cuts it off.
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

## Automatic texturing (`themes/kokiri.json`)

The document never names a texture. The builder classifies every piece of geometry and the theme maps
each class to a material, a tiling and overlays:

| Geometry | Rule | Kokiri |
|---|---|---|
| Floors, the bank | World-projected UVs: seamless across faces, regions and the bank | `ground`, 400 per tile |
| Pond beds, water | World-projected | `ground`; `water` at 54 per tile |
| Walls | First matching `wall_rules` entry: over water, then height | Shore `cliff_strip_dark`; up to 75 high `cliff_strip` (ledge); taller `cliff` |
| | A region's `edge` overrides the rules for its own walls | e.g. `vines` |
| Wall U | Arc length round the face, continuous round corners, **snapped to whole repeats round a closed loop**, so there's no seam | |
| Wall V | **Caps:** the texture's grassy top and bottom keep their own size on every wall, and a middle section repeats as many times as the height needs, so tall walls are as sharp as short ones. Walls no taller than the texture show it once. The middle runs from texel centre to texel centre and can be **mirrored** (every other repeat upside down), so repeats meet texel for texel. The repeat count is fixed per wall run, so neighbouring segments match | `cliff`: rows 0-4 top, rows 10-31 bottom, rows 5-9 (the flat dark rock under the lip) repeat, mirrored, at 167 units per texture height |
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
5. **Props and blocks**: houses, stumps, fences placed on floors. The editor stores where; a theme's kit says
   what.
6. **The editor** (`../overworld_editor`): built, with plan and 3D views, path profiles, the brush and Play.
7. **Play mode** (ADR 0035's consequences): the level as a mod pack so `Play_Init` enters it, actors placed
   in the editor, hookshot targets, exits between levels, and rooms for big levels.
