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

## Building it in slices (2026-10-25)

Steps 2 to 5 above (ROADMAP stage E5), cut into slices that each end in the editor with something
to try by hand: a hand-test list per slice, and the next slice planned around what that test turns
up. Storeys come first because they're what's new; seeing inside grows with them (each slice's 3D
cut, then the section cut once there are stairs to check). By the end of slice 4 you can build the
Deku Tree's central room (three floors, a hole through each, stairs and ladders between them) and
walk it in the game; slices 5 to 10 are the dungeon around it.

| # | Slice | What you can do at the end | Hand test |
|---|---|---|---|
| 1 | **Storeys, stacked and switched** | Add a storey above or below; a storey bar (2F, 1F, B1) picks the one you edit with the usual tools; the storey below shows faded in the plan; in 3D everything above the storey you're on is cut away; each storey is under the next one's slab; Play from here lands on the storey you're on; props and actors stand on their own storey's floor | Draw a two-storey room, put a prop and an actor upstairs, look at both floors in 3D, play from here on each |
| 2 | **Holes between storeys** (`open` regions) | Draw a hole in a storey's floor: the storey below has a matching hole in its ceiling, the slab's edge is a wall, a fence line round it is a railing | Cut a drop hole in 2F, fall through it in the game, land on 1F |
| 3 | **Stairs between storeys** | A path whose far end is on the storey above (the end's storey is picked in the inspector) climbs to it and cuts its own stairwell through the slab | Walk up from 1F to 2F and back down |
| 4 | **Ladders and vines between storeys** | A climbable face from one storey's floor up through a hole to the next (a ladder or vines on the hole's edge, carried down the wall below it) | Climb from 1F to 2F; build the Deku Tree's central room |
| 5 | **The section cut** | A line drawn across the plan shows the level side on: every storey's floors, ceilings and slabs, with a Link-height gauge; the builder warns where a stair or a floor leaves Link no headroom | Check a stairwell clears Link's head; make one that doesn't and see the warning |
| 6 | **Storeys over part of a level** | A storey with an outline of its own: a loft in a house, the shop under Zora's Domain's ledge, the passage behind the falls; the roof over a storey below is walkable outside it | A loft over half a room, walked onto by stairs |
| 7 | **Dungeon mode: rooms and doors** (editor only) | A mode where a storey is cut into rooms (regions with walls round them), and doors placed on a wall two rooms share: doorway, wooden door, shutter, locked, boss door, crawlspace | Lay out a three-room storey with a locked door, walk it in the game (still one room to the game) |
| 8 | **Rooms in the game** | The level exported as a scene of several rooms with transition actors: rooms load and unload as the game does them | Walk through the doors; the room behind loads |
| 9 | **The pause map, drawn** | The dungeon map page made from the storeys and rooms (ADR 0048's per-room images) | Open the pause map in the custom dungeon |
| 10 | **Flags and the completability check** | A links view handing out switch and chest flags (switch opens shutter, room cleared opens the doors); a check that every locked door has its key and everything is reachable from the entrance | Wire a switch to a shutter; remove a key and see the check fail |

### Slice 1's design

- **A storey is the level's content at one height.** The document's own `regions`, `paths`,
  `lines`, `props`, `actors`, `terrain` and the outline's `z`, `noise` and `ceiling` are one storey
  (named by `storey`, its slab `slab`); `storeys` holds the others, each with the same fields. Any
  storey can be the one in the document's own fields: storeys are ordered by height, not by where
  they're kept. So a level without storeys is unchanged, and the editor edits a storey by swapping
  it into the document's own fields (`Doc::swap_storey`), which keeps every tool, the inspector and
  undo working as they are. Switching storeys isn't an edit (no undo step, nothing to save); the
  file keeps whichever storey you were on in the document's own fields, which is the same level.
- **Every storey shares the outline** (slice 6 lets a storey have its own). A storey with one above
  it is enclosed: its ceiling is the underside of the slab under the next storey (`slab` thick, 60
  by default), and its roof (where a skylight's shaft ends) is the next storey's floor. The top
  storey's ceiling is its outline's own: none for a roof terrace open to the sky.
- **The builder builds each storey as a level of its own** and puts their meshes together
  (`Mesh::append`); problems say which storey. A level with no `storeys` builds byte for byte as
  before. The built level says where each storey is (`Level::storeys`: its floor, and the band of
  heights it fills), so the editor's plan, its 3D cut, the ground props and actors stand on, and
  the export each look at one storey's band.
- **Names:** the editor names storeys as it adds them, as the pause map does (the first storey 1F,
  then 2F, 3F up and B1, B2 down), and you can rename them; a storey without a name is called by
  its place in the stack.

### Slice 1: built (2026-10-25)

- `doc::Storey`, `Doc::storey`, `slab`, `storeys`, `swap_storey`, `stack`, `storey_doc`,
  `storey_name`, `SLAB`; `build::build_with` builds each storey with `build_one` and joins them
  (`Mesh::append`), `Level::storeys` (`StoreyBand`) and `Level::band`; `props::Ground::new_in` (one
  storey's ground); `Placed::storey`; the export puts each storey's actors on its own floor.
- Editor: the storey bar down the plan's left side (+ on top and underneath, a storey per button,
  right-click the one you're on for its name, floor height, slab and Delete), Ctrl+PageUp and
  Ctrl+PageDown to go up and down a storey; the storey underneath dashed in the plan; "under 2F ·
  ceiling 340" on the outline; the plan, its picking, the ghost and Play from here look at the
  storey you're on (`Scene::new`'s band); 3D cuts away what's over it ("Storeys above" shows it);
  the outline's Ceiling section says when a slab is its ceiling; a problem is clicked through only
  on its own storey.
- Example: `examples/sketch/sketch_storeys.json` (B1 with a pool, 1F with a dais, a stump and a pot,
  2F open to the sky with a loft, a stone and a pot).
- Tests: `storeys_stack_each_under_the_next_ones_slab` (the builder: heights, ceilings, closed
  edges, bands, each storey's ground, the same level whichever storey is in front, a too-thick
  slab), `storeys_are_added_named_moved_and_deleted` (the editor's helpers).
- **Not yet:** ▶ Play without a point starts Link on the highest floor in the middle (the top
  storey): use Play from here (right-click in the plan) on the others, until slice 3 gives the
  level a start. Only the storey you're on is pickable in 3D. A slab's edge isn't walled outside the
  outline (you see the gap from outside, with "Storeys above" on).

#### By hand

Done 2026-10-25: the floors show and switch as they should.

Open `crates/tools/overworld/examples/sketch/sketch_storeys.json` in the editor (Plan + 3D).

1. The storey bar reads +, 2F, 1F, B1, + down the plan's left; 1F is lit. The plan shows the dais,
   the stump and the pot; B1's pool is dashed, labelled B1; the outline says "under 2F · ceiling 340".
2. In 3D, 1F is open from above (2F cut away): the dais, the stump, the pot, the walls up to the
   ceiling. "Storeys above" puts 2F back on top.
3. Click 2F: the loft, the stone and the pot; 1F's dais dashed under it; in 3D the whole house,
   2F open to the sky with the forest round it. Ctrl+PageDown twice: B1 and its pool, nothing dashed
   under it.
4. Ctrl+Z after switching storeys undoes your last edit (taking you back to the storey it was on), not the switch; switching alone doesn't
   mark the level changed (no * in the title).
5. On 1F, draw a region and a prop: they're on 1F. Right-click 1F in the bar: rename it, raise its
   floor (the dais and what's on it go with it), thicken its slab (B1's ceiling comes down).
6. + on top: a 3F over 2F, empty, being edited. Delete it from its right-click menu: back on 2F.
   Ctrl+Z brings it back.
7. Right-click in the plan on each storey, Play from here: Link starts on that storey's floor
   (B1 beside the pool, 1F on the floor, 2F on the roof terrace). The pots are on their own floors.
8. A level without storeys: "+ Storey" only; click it and the level's storey becomes 1F with an
   empty 2F over it. Ctrl+Z: as it was.

### Slice 2: holes between storeys, built (2026-10-25)

- A region kind **`open`** (the editor's **Hole**): a hole in a storey's floor down to the storey
  under it. Its height is unused: `build_with` puts it at its storey's floor (`holes_down`), and
  `build_one` leaves it without a floor (`Info::open`, `Builder::hole`), walls standing where a
  higher floor (a dais) meets it.
- The storey under it opens there (`holes_under`): the hole is a region of that storey's too,
  open to the sky (`Ceiling::Sky`), as high as the floor it lies in, so nothing there changes
  height. Its ceiling has the hole, and the skylight's shaft (the column model's own wall) is the
  slab's edge, from that storey's ceiling up to the floor over it. A hole the same shape as a
  region under it opens that region instead (a pool under a well).
- A hole has to lie inside one floor of the storey under it: regions never cross, so one across a
  region's edge there is reported ("the hole ... crosses the edge of a region here") and the
  ceiling stays closed under it. On the bottom storey a hole is a pit, reported.
- Editor: the Hole card in the region palette, Hole in the inspector's Kind (with "Add a railing
  round it": a closed fence line `edit::RAILING_OUT` outside its edge, `edit::railing_round`), holes
  drawn in cyan in the plan, and the storey over this one's holes dotted ("2F's hole": where the
  ceiling opens); " · hole" in the outliner.
- Example: `sketch_storeys.json` now has a well through 2F (with a railing) over 1F's floor, and a
  drop through 1F into B1.
- Tests: `a_hole_in_a_storey_opens_the_one_under_it` (no floor, no ceiling under it, the slab's
  edge, closed edges; a hole the dais's shape; one across its edge; one on the bottom storey),
  `a_railing_stands_just_outside_a_hole`.
- **Not yet:** the floor under a hole is lit as if under the sky (it's not darkened as the rest of
  its storey is); a bridge across a hole (a floating path) isn't tried; a hole over a slope, or
  over a floor with bumps, takes the floor's flat height (and its bumps flatten at the hole's edge).

#### By hand

Open `sketch_storeys.json` again (Plan + 3D).

1. On 2F: the well in cyan with its railing round it, the loft; in 3D the well opens onto 1F's
   floor, its sides the slab's edge, and the railing stands on 2F round it.
2. On 1F: "2F's hole" dotted where the well is; the drop in cyan; in 3D (2F cut away) the ceiling
   has the well's hole in it, and through the drop you see B1's floor.
3. On B1: "1F's hole" dotted over the floor beside the pool.
4. Draw a hole of your own on 2F (palette: Region, Hole) over the dais: 1F's dais opens under it
   if it's the dais's shape, else it's open over the dais's floor. Draw one across the dais's edge:
   the problems list says it crosses, and 1F's ceiling stays closed there.
5. Select a hole: Kind says Hole; "Add a railing round it" puts a fence round it.
6. Play from here on 2F beside the well, walk in: Link falls to 1F. Walk into the drop: down to B1.
7. A hole on B1 (the bottom storey) is a pit: Link voids out, and the problems say so.

### Slice 3: stairs between storeys, built (2026-10-25)

Slices 1 and 2 committed and pushed (dbdf8b5).

- A path can go **up** (`Path::up`, the inspector's Climb section, "Up to 2F"): its last node is on
  the storey over its own, as high as that storey's floor there unless the node has a height
  (`build::stairwells`: the innermost loop of that storey round it; onto a hole, a pond or a pit
  is reported, as is going up from the top storey).
- It cuts its own **stairwell**: laid out as the builder lays it (`paths::layout`), the stretch
  where its surface comes within `STAIR_HEADROOM` (90) of the ceiling, up to where it's
  `STAIR_STEP` (10) under the floor it climbs to, `STAIRWELL_MARGIN` (10) wider each side, is a
  hole in that storey (`"open"`, named "stairwell (path)"), so slice 2 opens the storey under it
  and walls the slab's edge round it. The stairs' top runs on under the floor it climbs to (2
  under, `STAIR_TUCK`), so there's no gap: a step of 10 up at the stairwell's far edge. A
  stairwell crossing a region's edge on the storey over is reported and left out.
- `Level::stairwells`: drawn in the plan, the storey's own dashed in cyan ("stairwell"), the storey
  over's dotted ("2F's stairwell").
- **A level's start:** with storeys, `level.json` has `"start"` (`export::start`): the middle of
  the storey in the document's own fields (the one being edited when ▶ Play exports it), on its
  floor or the nearest one; `oot_import::level` spawns Link there. ▶ Play now starts on the storey
  you're on.
- Example: `sketch_storeys.json` has stairs up from 1F's west side to 2F, and from B1's east side
  up to 1F.
- Tests: `stairs_up_a_storey_cut_their_stairwell` (its top, the stairwell's span, no ceiling or
  floor over the stairs in it, the stairs under 2F's floor at its top, closed; up from the top
  storey); the start in `storeys_stack_each_under_the_next_ones_slab`.
- **Not yet:** the profile view under a path doesn't show the climb to the storey over (the
  builder sets the top's height); the stairwell meets the floor over it with T-junctions (no gap,
  but they can sparkle); stairs drawn as steps only where the theme has them (Kakariko's); stairs
  down (a path going up from the storey under is the same thing, drawn there).

#### By hand

Rebuild the game (the start is new in `oot_import`): `scripts\run\editor.bat` does it. Open
`sketch_storeys.json` (Plan + 3D).

1. On 1F: the ramp "stairs up" along the west wall, and "2F's stairwell" dotted over its upper
   half; in 3D (2F cut away) the ceiling opens over it. Select it: Climb says "Up to 2F: On".
2. On 2F: "stairwell" dashed at the ramp's top, beside the well; in 3D the ramp comes up through
   it, its top flush with 2F's floor.
3. On B1: "cellar stairs" up the east side, "1F's stairwell" dotted over it; on 1F that stairwell
   dashed in the south-east corner, beside the drop.
4. ▶ Play on 1F: Link starts on the dais in the middle. Walk up the west ramp to 2F, round the
   well's railing, and back down. Switch to B1 and ▶ Play: he starts on B1; walk up the cellar
   stairs to 1F.
5. Turn "Up to 2F" off: the ramp's top drops to 1F's floor and the stairwell goes. On again, then
   drag its top node into the well: the problems say it goes up into a hole. Onto the loft: it
   climbs to the loft's height, but its stairwell would cross the loft's edge, so the problems say
   there's none (stairs to a raised floor need the stairwell inside it).
6. On 2F (the top), a path with Up on: the inspector says there's no storey over it.
