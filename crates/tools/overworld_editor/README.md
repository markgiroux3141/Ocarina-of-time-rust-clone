# Overworld editor

The standalone editor around the `overworld` crate (roadmap item 6 in `../overworld/README.md`). Draw a level's outline, regions
and paths and place Kokiri's houses and features over a plan of the built level, set heights in a 3D view beside it and in a path's side profile. Every edit is
rebuilt in the background (under 0.1 s for the sketch levels), and **▶ Play** plays the build in the game as a scene of
its own, with the pad (`oot --level`, ADR 0035 and `docs/EDITOR-PLAY-IN-THE-GAME.md`), which reloads it whenever the editor
writes a new one.

```
cargo build --release -p overworld_editor -p oot
target/release/overworld_editor crates/tools/overworld/examples/sketch/sketch_village.json [--theme <theme.json>] [--textures <dir>] [--kit <dir>] [--select <name>]
```

`--select` selects a region or path by name, a prop by `prop:N`, a line by `line:N`, a region's edge by `edge:<region>:<k>` or an outline edge by `edge:outline:<k>`, at the
start (handy for screenshots: the profile shows for a path).

The texture libraries are `out/overworld/textures/<region>`, one per overworld region, made on first start from the
extracted scenes (`overworld kit-textures`; ROM data, never committed). So are the kits of pieces,
`out/overworld/kit/<region>`, cut from the same extracts by `overworld kit-pieces` (and again whenever a region's
manifest, `../overworld/kit/<region>.json`, changes); the editor loads them all as one kit. Play needs `target/release/oot` and the game's pack (its
first run imports it from the ROM in `oot.toml`). The build goes to `out/overworld/<level name>/`, which you can change
in the File menu.

## The window

- **Top bar:** the File menu (new, open, save, export, live export, and the theme, texture library and export folder),
  *Level settings…* (a window: name, theme, detail, edges, walls, in the game, the edge of the world, sampling), undo and redo. On the
  right: the build's status (click it for the triangles by object), the **collision meter** (vertices of the game's
  8192), the **⚠ problems** (click one to select and frame what it's about), the keys sheet (also `?` or F1), and
  ▶ **Play** with its ▾ menu (Link, the time of day, the save).
- **Tool rail** (far left): the tools as icons with their keys, grouped: select · region, path · brush · props ·
  rock, arch · dirt, fence, bridge, hedge, tunnel.
- **Palette** (left): what the current tool adds, and how. Select: which kinds a click in the plan picks (turn props
  off to click through to the floor under them, edges off to pick a region by its edge) and the main keys. Region:
  floor, water or pit, and how high a new one starts (a pit: how deep). Path: attached or floating, width, bridge shape. Brush: its six modes, size, strength and hard core, with
  its falloff drawn. Props: the kit as thumbnails, with a search, categories and the last four used. Dirt, fence,
  bridge, hedge: their look (swatches from the theme's own textures), and width or style for the next one drawn.
- **Inspector** (right): the selection's header card (its kind, its name to rename, delete), the build's problems
  with it, and its settings in sections. A value the theme supplies (a dirt path's width, a region's wall style)
  shows greyed with *theme* until you set your own; ↺ goes back to the theme's. A prop's turn has a dial.
- **Outliner** (right, below): everything in the level as a tree (terrain, paths, lines, props by kind), with the
  plan's colours, heights or kinds, and ⚠ on anything with a problem. Filter by name; click selects (Shift: several
  regions), double-click frames it in both views.
- **Views:** shading and nodes at the plan's top left, zoom and fit at its bottom right, plan / plan + 3D / 3D at the top
  right. The status bar has the tool's hints, the last message and the point under the pointer.

The thumbnails are drawn by the editor from the kit's own meshes and the texture library (`app/thumbs.rs`), once per
piece. The look is in `app/style.rs`: Segoe UI and Consolas where Windows has them, egui's own fonts elsewhere.

## Using it

- **Views:** *Textured* is the level from above, with its baked lighting, and higher ground drawn lighter. *Heights* colours
  floors by height, from dark green to pale rock. *Lines* shows only the document. The wheel zooms. Right-drag,
  middle-drag, or a left-drag on empty space pans.
- **Select (V):** click a node, path or region. Drag a node to move it, along with every loop node welded to it (shared nodes
  have a green ring). Drop a node onto another loop's node to share it, which is how a region is attached to the outline.
  Hold Alt to drag without snapping. Double-click a line to add a node: on an edge two loops share, the node goes into both.
  Delete removes the node, region or path. S makes a corner sharp or smooth again. PgUp/PgDn (or `]`/`[`) raise or sink the
  selected region by 20 (Shift: 100), or a path node's height.
- **Actors (C):** the game's own actors (docs/EDITOR-ACTORS.md). The palette lists every actor the port has, by group,
  with how often the game places it; pick one, and what it starts as (the game's own configurations for it), then click
  the ground in the plan or 3D to place it. Drag a marker to move it, its handle to turn it (Q / E), PgUp/PgDn to raise
  it, Ctrl+D to duplicate. The inspector shows its settings from its schema (named values, flags with who else holds
  them, items, drops), the flags it uses, the layers it appears in (child or adult, day or night), the game's
  configurations to take, and its raw params and rotation. Room objects (in the palette) counts what the actors need
  against the room's 16. Run it with `scripts\run\editor.bat` so it reads the scripts' pack and plays the game built
  beside it.
  The actors show as the game draws them (docs/EDITOR-ACTORS.md, ADR 0078): at the start, and whenever the level has an
  actor set up a way not captured yet, the editor runs the game beside it to capture what each actor's own draw code
  submits (`oot capture-actors`, into `out/editor/actors`; a few seconds, in the background). The 3D view draws each
  placed actor's capture with the game's renderer (its colours, faces and lights), the plan its outline from above, the
  palette and the inspector its thumbnail; an actor not captured in its own settings shows the nearest captured, an
  invisible one (a Navi spot, a song spot, a weather tag) the area it watches, and one that draws nothing a marker. The
  inspector says which, and why. Click a model in 3D to select it.
- **Moving a whole thing:** drag a line (a rock or a hedge from anywhere inside its shape, an arch, fence or dirt path along
  it) to move it whole, in whole units; it's selected as you do. Select a region or a path first, then drag it by its body
  (not a node) to move it whole: a region takes the nodes it shares with the outline or other regions along, so it stays
  attached; Shift-clicked regions move together. The outline doesn't move whole (that's the whole level): drag its nodes.
  A drag on empty ground still pans.
- **Edges:** a click near a region's edge (between two nodes) selects that edge; Shift-click more edges of the same
  region to select them together. The inspector then shows an *Edge* section above the region's own: its profile, the
  region's (the default) or one of its own: *Cliff*, *Slope* (angle, rounding), *Terraces* (steps, step height, depth),
  *Overhang* (depth), *Ragged* (how much, size, seed) or *Stack*. Del puts the selected edges back to the region's profile. The
  region's own profile, for all its edges, is in its *Edges* section. In the plan, a sloped edge has hachures pointing in
  (as maps draw slopes), terraces a second line just inside, an overhang a dashed line just outside, ragged rock a
  zig-zag. A profile is built inward from the edge, so the region keeps its shape; a sunken region's edges go up to the
  floor beside them. See Edge profiles in `../overworld/README.md`. An edge two regions share is the selected region's, else
  the higher one's: select the region first to give the lower one's side a profile.
- **Stacks:** a *Stack* profile shows Kakariko's edges as three cards, each with a cross-section: *Grass slope* (the west
  wing), *Rock face* (under Death Mountain Trail), *Mossy wall* (the south edge), in Kakariko's own textures whatever
  the level's theme. The one the stack is lights up.
  Its parts are under *Fine-tune* (closed until opened, open while the stack is none of the three): a list from the foot
  in, each a *Wall* or a *Slope* with its height (blank: a share of what's left), a slope's angle, and its *Style* (any
  theme's wall style; a slope with one is drawn as that wall). ↑ ↓ reorder them, *Remove* drops one, *+ Wall* and
  *+ Slope* add one. In the plan a stack has a line along its foot where it starts with a wall, and ticks where it
  slopes. See Stacks in `../overworld/README.md`.
- **Beyond the outline:** a click near an outline edge selects it (an edge a region shares with the outline is the
  outline's unless that region is selected). Its inspector section, *Beyond*, is *Forest* (the edge of the world as it
  was) or *Ground*: one of Kakariko's three edges (the cards; *Grass slope* to start) and its *Height* above the level's
  floor (the crest, or a skyline wall's top), which a card sets to Kakariko's. *Fine-tune* holds *Roughness* (how much
  the height rises and falls), *Skyline wall* (a style standing on the far edge: the mossy wall) and the whole profile
  (any kind, or the stack's parts). The ground is built outward from the edge, with nothing past its crest, and its
  ends slope down into the forest beside it.
  Shift-click more outline edges to set them together; Del puts them back to the forest. In the plan, blue ticks point
  out from those edges. See Beyond the outline in `../overworld/README.md`.
- **Region (R):** click points, then click the first point or press Enter to close. A click on an existing node shares it.
  A click on a loop's edge adds a node there and shares it, so a region can be drawn against the outline or another
  region. New regions start 120 above the ground they're drawn on. One drawn outside everything is a new **area**: it
  starts at the ground's height, with an edge of the world of its own (see Areas in `../overworld/README.md`); join it to
  the rest with a tunnel. A *Pit* (palette) is a drop into the void, its walls going down the palette's depth (600) below
  the ground round it: Link falling in voids out (see Pits in `../overworld/README.md`).
- **Level with what it's in** (palette, New regions, for floors; off by default): a new region starts at the height of
  the floor it's drawn on, the region it's inside, instead of Rise above it.
- **Ceilings**: a region's inspector has a Ceiling section: Inherit (under the ceiling of the region it's in), Open (a
  skylight) or Covered, with its height over the floor, a dome or vault (and how far in it rises), rough rock, the roof
  (automatic: the floor round it) and the style of the walls it adds (the ceiling is always the theme's ceiling rock). The outline's Ceiling section makes the whole level enclosed. In the
  plan a covered region has a dashed ring just inside its edge and its ceiling's height under its name, and the plan
  shows the floors under ceilings and roofs. In 3D ceilings only face down, so from above you see into the rooms under
  them; the **Roofs** button (top right, when the level has a ceiling) cuts the roofs over them away too, to see into
  caves dug into hills (see Ceilings in `../overworld/README.md`).
- **Fog**: the 3D view draws the theme's scene's fog as the game does (Zora's Domain's teal, Kokiri Forest's warm haze),
  into its colour, measured as if you stood 250 units in front of what the view orbits, so that spot looks as it does
  in play, zoomed out or not. The **Fog** button (top right) switches back to a light haze into a blue sky.
- **Path (P):** click points, then double-click or press Enter to finish. Put each end *inside* the floor it starts or
  finishes on, not on its edge: an end takes the floor's height. A ramp slopes all the way to its end, so one that
  ends halfway into a plateau cuts into it, and a bridge's floating end lands at the floor's edge (see the paths
  section of `../overworld/README.md`). An end exactly on an edge could take either side's height, so path clicks never snap
  to region nodes. The inspector sets width, mode per segment (attached embankment or floating bridge), surface (*Ground*,
  *Dirt*: the theme's dirt painted along it, or *Steps*: Kakariko's stairs, a ramp with steps drawn on it; drawn twice as long as it climbs its sides show the stairs'
  profile, any other slope plain brick repeating along them; a new path on Kakariko's theme starts as *Steps*), side style and
  bridge shape. A path node's height and width are optional, per node. Its *Section* card picks a cross-section
  (*Causeway*, *Sunken lane*, *Ledge*, *Boardwalk*, with their height, depth or post spacing; *Per segment* sets one
  per segment) and *Railings* (a fence kind, put up wherever a side drops away). Its *Climb* card turns on
  *Switchbacks*: between its two nodes it zig-zags across a *Corridor* at the *Slope* you give, with *Round* or *Flat*
  turns, its *First leg* heading left or right; the plan draws the zig-zag. An end put on another path joins it (see
  Junctions in `../overworld/README.md`).
- **Brush (B):** paints the level's terrain (see Painted terrain in `../overworld/README.md`): one smooth height offset that
  everything rides on, so painting across several regions raises them all together, with their walls, paths and bridges.
  Modes: Raise, Lower, Smooth, Flatten, Bumps, Erase (keys 1-6). Ctrl turns raise into lower; Shift smooths. `[` and `]`
  size it; *Hard core* is the share of the radius at full strength. Paint in the plan or in 3D (there, Alt-drag or
  middle-drag orbits). Each stroke is one undo step. The rebuild follows a few times a second while you paint.
- **Prop (K):** the palette shows every region's pieces as thumbnails (about 200, from Kokiri Forest to the Desert
  Colossus). *Region* narrows them to one region's; the chips below narrow them to a kind (houses, buildings, posts,
  bridges, platforms, stones, rocks, plants, decor, openings, on walls: only the kinds there are), and the search
  matches names and regions. Hover one for its region, size, triangles, collision vertices and what Link can use it for. Click in the plan or 3D to place the chosen piece; it
  stands on the ground (a house on its doorway, a stone in water). Drag a prop to move it, in either view. In the plan,
  the selected prop has a handle on its facing arrow (drag: turn, in 15° steps) and one on its corner (drag: scale,
  within the piece's limits). Alt: no snapping. Q / E turn it 15° (Shift: 1°), PgUp/PgDn raise or sink it (it then
  keeps its own height; "on the ground" puts it back), Ctrl+D duplicates it, Delete removes it. The inspector sets its
  position, height, turn and scale, and tells what Link can use (door, ladder, crawl...). Doors and the log tunnel's exit
  are scenery until levels load through `Play_Init`. See Props in `../overworld/README.md`.
- **Dirt (D):** click points along a dirt path, then double-click or press Enter. It's painted into the floor (see Dirt
  paths in `../overworld/README.md`): the ground under it fades to dirt across its soft edge. Its nodes drag like a
  path's, a double-click on it adds one, and the inspector sets its name and width.
- **Fence (G):** click points along a fence (straight between them, a post at each), then double-click or Enter. The
  palette and the inspector switch between rails (40 tall) and the lattice (120), and can close it back to the first node.
- **Bridge (H):** click a point on each floor the bridge joins (anywhere on it: the ends land at the floor's edge), then
  Enter. A hanging rope bridge sags between them. The inspector sets its width, and a deck too steep to walk is reported.
- **Hedge (J):** click the corners of a patch of tall grass, then click the first one or press Enter, as for a region.
  Link wades through it. Its nodes drag like a fence's, and a double-click on its edge adds one. See Hedges in
  `../overworld/README.md`.
- **Tunnel (U):** click on the floor in front of the wall it goes into, any bends, then on the floor beyond the far wall
  (or in another area), then double-click or Enter. Its mouths go where the line meets a wall taller than the tunnel,
  walking in from each end; in between it follows the nodes. The palette sets the next one's width, height and rough
  walls; the inspector the same, with *Rough walls* (how much, size, smooth mouths, seed), and a middle node's *Floor
  height* (blank: a straight slope between the mouths). The plan draws it as a dark band where it was built, from mouth
  to mouth. A wall too low, too little ground over it or too tight a turn is reported. See Tunnels in
  `../overworld/README.md`.
- **Rock (O):** click round its footprint, then click the first point or press Enter, as for a region. Afterwards a click
  anywhere inside it selects it, and a drag there moves it. It starts in the
  palette's shape (cards with their silhouettes: *Boulder*, *Mesa*, *Spire*, *Mushroom*, *Pillar*), its heights grown to
  its size. Its inspector's *Shape* has the same cards (one click reshapes it), and under them a **side view** of it, as
  it's lofted, with a handle on each contour: drag one up or down for its height, in or out for its width (Shift: finer).
  *Fine-tune* lists the contours as numbers (height, scale, a shift that leans it) with *+ Contour* and *Remove*.
  *Surface* has its own lumps (else the theme's), *Layers* (grooves every so often) and its *Style* (any theme's wall
  style). PgUp/PgDn make it taller or shorter, its contours keeping their proportions. In the plan its footprint is solid
  and its contours dashed inside it (outside where they overhang), the top one bolder, with its height by its name. Its
  nodes drag like a hedge's, and a double-click on its edge adds one. See Rocks and arches in `../overworld/README.md`.
- **Arch (A):** click where each foot stands (points between bend it), then double-click or Enter. The palette and the
  inspector set its *Height* (the top of its crown over its feet), *Width* (across its top) and *Thickness* (at its
  crown), each the theme's until set, and the inspector its *Segments*: how many pieces it's built of along it (few for a
  faceted arch, more for a smooth one; blank shows, greyed, the count the level's detail gives it); *Surface* its lumps
  and style. PgUp/PgDn raise or lower its crown. The plan draws
  its width as a band with a tick across its crown. No room under it, or feet closer than its width, is reported.
- **Openings and wall pieces:** in the Prop tool with the log tunnel, the crawlspace, the vine patch or the waterfall
  chosen, hover a wall, best in the 3D view: a green ghost shows where it would go (fitted to that wall, slid clear of
  corners, with how wide the wall is), or a red ring says why it can't (too low, not flat, not parallel, no room behind).
  Click to put it there. The plan shows the same ghost from above. Drag one along the walls in 3D to move it (the ghost
  follows). An opening cuts its gap (see Wall openings in `../overworld/README.md`). A crawlspace needs a ridge with flat,
  parallel walls: draw it with sharp corners (S), or with Hard edges. Vines and the waterfall always reach from the floor
  to the wall's top; the inspector's *Width* sets how wide they are, up to the flat face they're on (it says how wide that
  is). The vines' texture repeats as they grow.
- **Themes:** Level settings' *Theme* (a list of the 19 regions with themes) switches the level's look: every floor, wall
  and tree that isn't pinned to a style follows. A region's *Edge* and a path's *Sides* list the level's theme's styles
  first, then the other themes' (`kakariko:brick`, `kokiri:cliff`), which stay as they are when the theme changes. A
  style the theme doesn't have falls back to its own walls, with a problem saying so. The texture libraries of every
  theme are made from the extract on first start (`out/overworld/textures/<theme>`). Kakariko ends at the sky: switching
  to it gives an outline that's forest all round Kakariko's *Grass slope* beyond every edge (and switching away, while
  every edge is still just that, the forest again; `edit::theme_switched`). See Themes in `../overworld/README.md`.
- **New levels** start at Low detail, Hard edges and Stretched walls (File → New, and the editor with no file). A level
  without these settings in its file keeps High, Smooth and Tiled.
- **In the game:** Level settings' *In the game* (the document's `game`, exported into `level.json`): how the level
  plays when ▶ Play enters it. *Start from* picks the region whose game dynamics it plays with (its music and nature
  ambience, the HUD's rules, Navi's hints, the camera type, and its first room's hazards and clock); by default the
  theme's. The theme still sets the look: the lights, fog and sky. Then changes to it: *Music* (another region's, or
  none), *Hot* (the heat timer, as in the crater), *Sky* and *Clock*, each *Start's*, *On* or *Off*. A change while the
  game is open reloads the level with it, rules and all. Nothing set, the document saves as before.
- **Edges:** Level settings' *Edges* (next to *Detail*) sets how every outline, region and path runs between its
  nodes: *Smooth* curves, *Faceted* (the curves in a few flat panels) or *Hard* (straight node to node, so the
  nodes you place are the corners). See Edges in `../overworld/README.md`. *Walls* switches the cliffs between
  *Tiled* (caps at their size, the rock between repeating), *Middle stretched* (caps at their size, the rock between
  stretched once) and *Stretched* (the texture once over each wall's height, as Kokiri's own walls are).
- **Several regions:** Shift-click regions (in the plan, in 3D or in the outliner) to select several. PgUp/PgDn, the inspector's
  -20/+20 and dragging in 3D raise or sink them all together; Delete deletes them all.
- **Bumps:** a region's inspector (and the outline's) has *Bumps*: height, size, edge fade and seed. See the Bumps section
  of `../overworld/README.md`. The Heights view shows them best from above, and the 3D view or Play up close.
  Houses, stumps and hedges level the bumps under them, rebuilt with every move; a prop's inspector has *Level* to turn
  that off (or on for another kind). Only while it's on the ground.
- **Path profile:** selecting a path opens its side view under the plan, laid out by the builder's own `paths::layout_all`
  (so an end joining another path shows the height it takes there, and switchbacks show as built, their two nodes at
  the ends).
  You see the ground under it, embankments filled down to the ground, bridges in blue, and each segment's slope, with
  anything steeper than the theme's walkable slope (35°) in red. Drag a node up or down to set its height (5-unit
  steps; Shift: 1). Right-click it for "Automatic height". ×1 shows true slopes; ×2 and ×4 exaggerate the heights. The
  pointer's spot along the path is marked on the plan and in 3D.
- **3D view** (Plan + 3D, or 3D, in the top bar): the build in perspective, textured with its baked lighting as the game
  draws it. Drag to orbit, right-drag to pan, wheel to zoom; a double-click orbits round that spot. Click a region or
  path to select it, then **drag the selected region up or down to raise or sink it** (5-unit steps; Shift: 1). The
  selection is outlined at its height straight away, and the level catches up when the rebuild lands. The selection is
  drawn in orange over the view: a region round its edge, a path along it, a rock as a cage of its footprint and contours,
  an arch along its top, any other line along the ground, a prop as a box, a node as a post standing on it. F frames
  both views.
- **Play:** ▶ Play exports the build and starts the game (`oot --level <out dir>`): the USB N64 pad, or the keyboard
  (WASD, Space A, E B, Q Z-target). The level is entered by `Play_Init` as a scene of its own (a new scene id after the
  game's), with its theme's scene's header: its music and nature ambience, sky, lights and fog, keep object, Navi's
  hints and HUD restrictions, and its first room's behaviour (a crater-themed level is hot: the heat timer runs). So it
  plays as the game does, with the HUD, Navi, the clock, the pause menu, effects and void-outs. The ▾ menu beside Play
  sets child or adult Link, the time of day and the save (a new one, or the sword and shield, sticks, the slingshot and
  nuts); a change restarts the game at the next Play. W plays from the cursor, and so does the right-click menu's "Play
  from here", facing north (the spawn goes there, so a void-out returns there). While the game is open, every rebuild is
  exported and it reloads the level with Link where he stands (`Play_Init` again: the music carries on, the actors start
  over; not while the pause menu is open). The level has its surface types (grass, no-grab cliffs, vines) and a water
  box for each pond. A level whose collision needs more than 8192 vertices is refused: build it at Medium or Low.
- Undo/redo: Ctrl+Z / Ctrl+Y. Each drag or each field edit is one step. Save writes the document with each node on one line.

**Detail:** Level settings has High / Medium / Low (see Detail in `../overworld/README.md`); click the build status in
the top bar for each object's triangles. The **collision meter** beside it shows the vertices the game's collision will
hold, of 8192 at most (amber near it, red past it); its popup has the props' share. A failed build shows the last good
one, says why under ⚠ and in a red banner across the bottom of the views, and rings the spot the error names (such as
loops crossing) in the plan.

## Code

- `edit.rs` (no UI, unit tested): welded node groups, moving, inserting on shared edges, deleting, picking, the curves the
  builder draws (`overworld::map::sample_loops`, `overworld::paths::centre_line`), and the JSON writer.
- `worker.rs`: builds on a background thread, skipping documents that are already stale, and exports with
  `overworld::export::write`.
- `scene.rs`: the plan camera and the level drawn from above (egui meshes, lowest triangles first), plus floor heights
  under the cursor and ray casts against the floors (3D picking).
- `profile.rs`: a path's side profile, with the painted terrain included (node drags convert back to design heights).
- `view3d.rs` + `view3d.wgsl`: the 3D view, using the window's wgpu device (via `eframe::egui_wgpu`) to render offscreen
  with 4x MSAA and mipmapped textures, shown as an egui image, the way the OoT Clone's `oot_viewer` does it. The placed
  actors' captures are drawn in the same pass by the game's renderer (`eng_render`), and their thumbnails with it.
- `actors.rs`: the actors' catalogue (the port's profiles, the palette's names and groups, the game's placements) and the
  invisible actors' areas. `captures.rs`: the game's captures of the actors (running `oot capture-actors`, reading the
  manifest and the files, the nearest captured configuration, outlines from above, areas as meshes).
- `app.rs`: the editor's state, tools, plan and 3D input, drawing the plan, undo, files and Play; its panels in `app/`:
  `chrome.rs` (top bar, tool rail, status bar, view overlays, keys sheet, level settings), `palette.rs`,
  `inspector.rs` (with a rock's shape cards and side view), `outliner.rs`, `widgets.rs` (sections, rows, segmented choices, cards, chips, theme-default fields),
  `icons.rs` (line icons drawn with egui shapes), `style.rs` (colours, fonts, egui visuals) and `thumbs.rs` (the kit's
  thumbnails, by a small software rasteriser).

Not yet: editing nodes in 3D; props' handles in 3D (move works there; turn and scale are in the plan, the inspector and Q / E).
