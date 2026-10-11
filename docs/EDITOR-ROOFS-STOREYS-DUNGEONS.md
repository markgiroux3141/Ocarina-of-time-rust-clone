# Level editor: roofs, storeys and dungeons

A plan from 2026-10-10 for growing the overworld editor (`crates/tools/overworld`, `crates/tools/overworld_editor`) past
open-sky levels: places with a closed roof (caves, grottos, interiors, Zora's Domain), places with several floors
(the Deku Tree's central room, houses with lofts), and in the end whole dungeons. The overworld ideas are in
[OVERWORLD-EDITOR-IDEAS.md](OVERWORLD-EDITOR-IDEAS.md); this document carries on from its "Caverns and holes".

## Progress

- **2026-10-10, step 1 (ceilings), first cut:** regions and the outline can have a ceiling (flat, domed or vaulted,
  rough) with the solid above it up to a roof; skylights; covers nest (a hut in a cavern); an enclosed level (the
  outline's ceiling) has walls up to its ceiling instead of the edge of the world. Ceilings are each theme's own
  seamless rock (2026-10-11), and every custom level is played in its theme's scene's lights and fog (Zora's Domain's
  teal haze for a Zora's Domain level), not Kokiri Forest's.
  Props and fences in a cave stand in it, not on the hill over it. Editor: a Ceiling section in the inspector, covered
  regions marked in the plan (which shows the floors under roofs), and in 3D ceilings face down only, with a Roofs
  button to cut the roofs away (a first piece of step 2). Built on the column model above; levels without ceilings build
  byte for byte as before. See Ceilings in the builder's README; examples `sketch_caves.json` and `sketch_cavern.json`.
  Left for later: a cutaway's cut edges aren't capped (you see through the solid there); ceilings take the theme's
  rock, which in Kokiri's theme is its banded cliff texture.

## Where we start from

The editor's grammar (see the ideas doc's "The test for a new feature") is **2.5D**: the plan is one web of loops,
each face takes one floor height from the innermost loop round it, and every wall is derived from neighbouring faces
whose heights differ. That's why levels stay watertight and why moving a plateau carries everything on it.

The only things that already break "one surface per (x, y)" are laid over the plan: bridge decks, arches, overhangs
and **tunnels**. A tunnel is already a space with a ceiling, swept along a line, so roofs aren't a new kind of
modelling. Three ideas are added on top of the grammar, each useful on its own.

## The column model (the core decision)

Each face becomes a **column**: the open intervals of air above its point, bottom to top.

| Face | Column |
|---|---|
| Open ground today | `[floor, ∞)` |
| A cave or a room | `[floor, ceiling]`, then `[roof, ∞)` above the solid over it |
| A skylight in a cave | `[floor, ∞)`, but its neighbours are `[floor, ceiling] ∪ [roof, ∞)` |
| A hut inside a cavern | `[floor, ceiling] ∪ [roof, cavern ceiling] ∪ [cavern roof, ∞)` |
| An enclosed level's ground | `[floor, ceiling]`; outside the outline nothing (solid) |

- **Surfaces:** each interval's bottom is a floor facing up (the first is today's floor; the others are roofs Link can
  walk on: the hill over a cave), and each finite top is a ceiling facing down. Both reuse the face's floor
  triangulation, so they share its edge points.
- **Walls:** on each edge, the wall facing a side is that side's air minus the other side's air. With no ceilings
  that's `[low floor, high floor]` facing the lower side: exactly today's walls, so older levels build the same.
  With ceilings the same rule gives the cave's walls, the lintel over a cave mouth (`[ceiling, roof]` facing out
  where the cave opens onto lower ground), a skylight's shaft and the mound of a cave whose roof stands above the
  ground round it.
- Today's wall machinery (styles, profiles, stacks, bends) keeps the floor-to-floor piece; the column rule adds the
  pieces above it and clips the floor piece at the lower side's ceiling.
- **Storeys are more intervals,** so step 3 doesn't need a new model.

### A cave into a cliff, built from the rule

A plateau at 600 with a region drawn inside it at floor 0, ceiling 300 (its edge partly on the plateau's edge):

```
            roof = the plateau's floor (600) — the hill goes on over the cave
   ─────────────────────────────┐
   solid                        │ lintel [300, 600] facing out (where the cave meets the lowland)
   ┌──── ceiling 300 ───────────┘
   │ cave [0, 300]                  lowland [0, ∞)
   └────────────────────────────────────────────
```

Inside the cave the walls are `[0, 300]`; where the cave's edge lies on the plateau's edge the lowland's air covers
the cave's, so there's no wall: that's the mouth.

## Step 1: ceilings (closed roofs)

- A region (or the outline) has a **ceiling**: none (it inherits its parent's: a ledge inside a cave is under the
  cave's ceiling), `sky` (open, a skylight inside a covered parent), or `closed`:
  - `height` above the region's floor (so moving the floor carries the ceiling: non-destructive);
  - a shape: `rise` (a dome or vault up to this much higher, over `reach` from the edge, or the whole region), and
    `noise` (rough cave rock, fading out at the edges, as floor bumps do);
  - `roof`: the solid's top. Defaults to the parent's floor (a cave dug into a hill keeps the hill), never less than
    a little over the ceiling;
  - `style`: the ceiling's and its walls' wall style (the theme's rock by default).
- The ceiling's shape is a field over its **owner** loop (the region whose ceiling it is), not over each face, so
  regions inside it and terrace treads stay under one continuous ceiling.
- **Enclosed level:** the outline's ceiling. No rim, bank or trees; the outline's edges are walls up to the ceiling.
  A skylight's shaft goes up to the outline's `roof` and is open to the sky there.
- Under a ceiling, everything is darker (a tint baked into the vertex colours).
- **Covers:** grottos, the Kakariko well, Dodongo's Cavern's rooms, house interiors, Zora's Domain's dome, Goron City's
  shell, caves in hillsides (the Death Mountain Trail cave, the Gerudo Valley crate cave).
- **Not in the first cut:** paths, rocks and props poking through a ceiling aren't checked; edge profiles (slopes,
  terraces) under a ceiling climb straight through it; ceilings aren't affected by the brush (painted terrain moves the
  whole column, ceiling too).

## Step 2: seeing inside

- **Dollhouse 3D view:** ceilings are one-sided, facing down, so seen from above they vanish (backface culling), and you
  look into the rooms. A "hide ceilings" toggle besides.
- **Section cut** (from the precision tools in the ideas doc): a vertical slice through the level showing floors,
  ceilings, roofs and a Link-height gauge. With storeys it's the only way to check that a stairwell clears Link's head.
- The plan view marks covered regions (hatching) and labels ceiling heights.

## Step 3: storeys (several floors)

- A level holds a stack of **storeys**: each one a full plan (the same tools), at a base height, with a slab
  thickness. A storey's slab underside is the ceiling of the storey below.
- A new region kind **`open`**: like a pit, but the drop leads to the storey below instead of the void. Atriums, drop
  holes, the Deku Tree's central room (three storeys with a hole in each, the web over the top one). The rim of the
  hole is a slab face; a fence line gives it a railing.
- **Connectors:** a path whose ends are on different storeys is a stair or ramp, and cuts its own stairwell in the slab
  over it; vines and ladders span storeys as wall pieces; switchbacks that change storey for spiral ramps (Zora's
  Domain, Goron City).
- Editing: a **storey bar** (3F, 2F, 1F, B1, as the pause map has them). The storey below shows ghosted and the one
  above as outlines.
- Zora's Domain is mostly one storey under a dome; storeys are only needed where something really is over something
  else (the shop under the king's ledge, the passage behind the falls).

## Step 4: Dungeon mode (rooms and portals)

A different top level, not a different way of modelling, so this is where an **alternate mode** pays for itself.

- A dungeon is a **graph of rooms on storeys**. A room is an outline (its walls) with a ceiling; inside it, the same
  regions, pits, water, paths and props as outdoors.
- **Doors are portals** on a wall two rooms share, typed: an open doorway, a wooden door, a shutter (bars), locked
  (small key), the boss door, a crawlspace.
- One for one with the game: a room = a room mesh; a door = a transition actor with its front and back rooms; room
  loading draws the current room and its neighbours; the pause map works by storey and room. The editor can **draw
  the dungeon map page**, rasterising each storey's room outlines into the per-room CI4 images the pause map
  (ADR 0048) already reads.
- Natural dungeons (Deku Tree, Dodongo's Cavern, Jabu-Jabu) fit the loop grammar as it is. Built ones (the Forest and
  Water Temples) want **grid snapping and hard edges** (already a setting) plus a dungeon kit of props (pillars, door
  frames, torches), not brush (CSG) modelling, which the ideas doc keeps out.
- **Dungeon themes** cut from the extracted dungeon scenes (ydan, ddan, bdan, ...), as the 20 overworld region kits
  were.

## Step 5: what a dungeon needs beyond its shape

- **Flags wiring:** switches, chests, "room cleared opens the shutters", torches lit: a small links view that hands out
  switch and chest flags itself.
- **A completability check:** small keys against locked doors, reachability from the entrance. In the ideas doc's
  spirit: exact, not guessed.
- **Volumes as regions:** camera settings (bg cameras per polygon), dark rooms and room lights, lava and ice,
  hookshot targets.

## The game's side

- Anything past one room needs what ADR 0035 queued: the level written as a mod pack (scene and room records, entered
  through `Play_Init`), transition actors and exits.
- **To check:** in OoT, collision is one header per **scene**, not per room, so rooms alone don't lift the 8192
  collision-vertex limit ADR 0035 mentions. Most dungeons fit under it; if one doesn't, the custom format widens or the
  dungeon splits into scenes. Ceilings and roofs count against the limit too (each is a copy of the floor's
  triangulation), so covered levels may need a lower detail sooner.

## Order

1. Ceilings (the smallest change, and caves, grottos, interiors and Zora's Domain follow from it)
2. Seeing inside: dollhouse view, hide ceilings, the section cut
3. Storeys, `open` regions, stairs between storeys
4. Dungeon mode: rooms, portals, the mod pack export, the generated pause map
5. Flags, the completability check, dungeon kits
