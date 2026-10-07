# Overworld editor: ideas for more varied levels

A brainstorm from 2026-10-06, to come back to. Nothing here is built or decided. The editor and builder are
described in `crates/tools/overworld_editor/README.md` and `crates/tools/overworld/README.md`; their own roadmap is at
the end of the builder's README.

## The aim

- **Flexible, but not Blender.** Enough tools to make OoT's variety of places, without free mesh editing.
- **Non-destructive.** You can go back and change anything, and what depends on it follows.
- **Positions and distances are exact.** Heights, gaps and slopes are what make a level play right.

## The test for a new feature

The editor already has a grammar: **everything is a 2D line or loop of nodes, plus a few numbers, and the builder
derives the 3D from it.** That's why moving a plateau carries its ramps, bridge ends, prop pads and wall openings with
it. A new feature fits if:

1. it can be described as lines or loops plus a handful of numbers;
2. it survives its neighbours moving (it's rebuilt from the document, not stored as geometry);
3. the level still builds watertight with it.

The theme chooses the look, so the document never names a texture. Biome variety (rock, sand, snow) belongs in
themes, not in extra tools.

## What OoT has that we can't build yet

| Place | What's missing |
|---|---|
| Hyrule Field | Gentle slopes instead of walls, rolling hills, a river |
| Death Mountain Trail | Cliff-side paths, boulders, overhangs, a switchback climb |
| Zora's River | A flowing river stepping down through falls and rapids, ledges along the canyon walls |
| Gerudo Valley | A chasm (a pit with no floor), a narrow ledge path, standalone rock arches |
| Goron City | A hollow ringed by concentric terraces |
| Lake Hylia | A big lake, islands, a pillar, platforms on stilts |
| Kakariko, Sacred Forest Meadow | Stairs, freestanding walls and gates, a hedge maze |
| Lost Woods | Corridors walled by trees, not cliffs |
| Haunted Wasteland, Desert Colossus | Dunes, quicksand, flag-post lines, mesas |
| Grottos, Zora's Domain | Holes in the floor, caverns with a ceiling |

## 1. Shapes: rocks and landforms

- **Lofted rocks, from contour loops.** Draw a rock's footprint, then add 1 to 3 more loops at chosen heights; the
  builder lofts between them, with noise, a seed and optional layering. That gives:
  - boulders, spires, mesas and buttes;
  - overhangs and mushroom shapes, when an upper loop is wider than the one below;
  - Lake Hylia's pillar, Desert Colossus's rocks.

  It's the region tool again, with heights dragged in a profile. No sculpting.
- **Contour hills.** Draw closed contour loops at heights and the builder fits smooth ground through them. This covers
  the big shapes the brush is used for now, without the brush's problem (see Non-destructive editing).
- **Standalone rock arches.** The rock arch under floating paths already exists; let it stand alone between two points.
- **Edge profiles, per stretch of an edge.** This extends the builder roadmap's soft edges. Each stretch picks one:
  - cliff, as now;
  - slope at an angle (Hyrule Field);
  - terraces (a number of steps of a given height);
  - overhang (the top juts out by some amount);
  - ragged rock (noise on the wall face).

  A region could be a cliff on one side and a gentle slope on the other.
- **Terraced regions.** One loop, a step count and a step height: Goron City's rings, or terraced hillsides.
- **Pits and chasms.** A region kind with no floor, where Link voids out: Gerudo Valley's canyon, the drops in Death
  Mountain Crater.

## 2. Pathways

- **Path cross-sections.** Each path segment picks a section, as it now picks attached or floating:
  - **cliff-side ledge:** cut into a wall on one side, dropping off on the other (Death Mountain Trail, Gerudo Valley);
  - **sunken lane:** a cutting with walls on both sides (Kakariko's roads);
  - **causeway:** raised, with optional railings;
  - **steps:** a step height, with the run solved from the slope;
  - **boardwalk:** planks on stumps or posts at a spacing (Kokiri Forest's walkways, made parametric).
- **"Make it walkable."** Give two endpoints and the builder adds switchbacks so the slope stays under the walkable
  35°. You choose the corridor width and turn style; it solves the zig-zag. You state the constraint, not the geometry.
- **Junctions.** Paths that share a node merge cleanly (Y and T joins), rather than overlapping.
- **Ledges along a wall.** A line drawn along a wall face, with a height and a width, becomes a shelf to walk or shimmy
  along (Zora's River, Gerudo Fortress).
- **Stepping-stone lines.** Stones placed along a line at a spacing taken from Link's own jump distance, so every gap
  is jumpable.

## 3. Water and other liquids

- **Rivers.** A line with a width, a flow direction and node heights. Where the height drops, it builds rapids or a
  waterfall, with the falls set on the walls they cross. The game needs current support for it (Zora's River pushes
  Link along).
- **Liquid kinds.** Lava, quicksand and swamp, as theme materials plus surface types on the existing pond machinery.
- **Lakes with shape.** A depth profile towards the shore rather than a flat bed, and islands.

## 4. Non-destructive editing

**Painted terrain is the one exception today.** Its offsets live in world-space chunks, so moving a plateau leaves its
painted hill behind. Ideas, from smallest to largest:

- Brush **layers** you can toggle, fade or delete, so a stroke can be taken back later.
- Paint **attached to an object**, stored in that region's own frame so it moves with it.
- Contour hills (section 1), which cover most big shapes, leaving the brush for touch-ups.

Other ideas:

- **Relative values.** A region's height as "Lower Ledge + 80" rather than an absolute 240; a prop's position as "12
  along this path" or "on this wall". Change the base and everything on it follows. Light constraint solving (one
  parent, one offset), not full CAD.
- **Rules, not copies.** Store "scatter graves in this area, spacing 120, seed 4" or "a torch every 300 along this
  path" as rules, so edits re-spread them. Arrays along a line, scatter in an area, and mirror across an axis (the
  Temple of Time's plaza, Gerudo Fortress).
- **Groups and prefabs.** Save a cluster (a house, a fence, a dirt path, two stumps) and place it again as linked
  copies: change the prefab and every copy changes.
- **Layers, locks and visibility** in the outliner, for when levels get dense.

## 5. Precision: positions and distances

- **Measure tool.** Click two points (in the plan or in 3D) for the horizontal distance, the height difference and the
  slope. A measurement can be pinned, so it stays on the plan and updates as things move.
- **Link gauges.** An overlay of what Link can do, with the numbers read from the player code (`player.rs`), not typed
  in:
  - jump gap;
  - ledge heights he can grab or climb, child and adult;
  - hookshot and longshot range;
  - the walkable slope.

  Then show it on the level: every wall coloured climbable, grabbable or too high; every gap jumpable or not. This is
  the visual half of the builder roadmap's reachability checks.
- **Grid and snaps.** An optional grid (10, 50 or 100), angle snap while drawing (15°), and typed length and angle as
  you draw (type 400, Enter).
- **Section cut.** Draw any line across the level and get a side profile, like a path's profile but for any cut. The
  quickest way to check whether this ledge is above that one, and by how much.
- **True-scale reference underlay.** An extracted OoT scene's collision (Hyrule Field, Kakariko) shown faintly under
  the plan at true scale, to size things against the real game. Also an image underlay, calibrated with two points and
  a known distance, which pairs with `tools/trace_sketch.py`.

## 6. The edge of the world and enclosures

- **Boundary types per outline stretch:**
  - forest, as now;
  - mountain wall (rock rising to the sky);
  - water horizon (Lake Hylia);
  - town wall;
  - an invisible wall with scenery beyond.
- **Tree walls as a line kind.** The boundary's trunks and foliage along any line, for Lost Woods corridors and clumps
  of trees inside the level.
- **Freestanding walls.** A line with a height, a thickness, a material (stone, wood, hedge) and gaps for gates: a
  hedge maze, Kakariko's walls, the castle town wall.
- **Caverns and holes.** A region with a ceiling (an enclosed cave room), and tunnels that start in a floor (grotto
  holes; already under the tunnels' "Not yet").

## What to keep out

So the editor doesn't become Blender:

- free vertex or mesh editing;
- mesh booleans;
- UV or texture painting per face;
- sculpting beyond the brush.

## If we had to pick five

1. **Lofted contour rocks:** arbitrary rock shapes and overhangs, from a tool we already know.
2. **Edge profiles per stretch**, including slope, terrace and overhang: the biggest change in how levels feel, and
   already on the roadmap.
3. **Path cross-sections, the cliff-side ledge and "make it walkable":** most of OoT's memorable routes.
4. **The measure tool, Link gauges and the section cut:** positions and distances become checkable, not guessed.
5. **Relative heights and brush layers:** they close the remaining gaps in non-destructive editing.

Rivers and the scatter and array rules would come right after.
