# Overworld editor

The standalone editor around the `overworld` crate (roadmap item 6 in `../overworld/README.md`). Draw a level's outline, regions
and paths over a plan of the built level, set heights in a 3D view beside it and in a path's side profile. Every edit is
rebuilt in the background (under 0.1 s for the sketch levels), and **▶ Play** plays the build in the game with child Link
and the pad (`oot_sandbox --level`, ADR 0035), which reloads it whenever the editor writes a new one.

```
cargo build --release -p overworld_editor -p oot_sandbox
target/release/overworld_editor crates/tools/overworld/examples/sketch/sketch_paths.json [--theme <theme.json>] [--textures <dir>] [--select <name>]
```

`--select` selects a region or path by name at the start (handy for screenshots: the profile shows for a path).

The texture library is `out/overworld/textures/kokiri`, made on first start from the extracted Kokiri Forest scene
(`overworld kit-textures`; ROM data, never committed). Play needs `target/release/oot_sandbox` and the game's pack (its
first run imports it from the ROM in `oot.toml`). The build goes to `out/overworld/<level name>/`, which you can change
under Files.

## Using it

- **Views:** *Textured* is the level from above, with its baked lighting, and higher ground drawn lighter. *Heights* colours
  floors by height, from dark green to pale rock. *Lines* shows only the document. The wheel zooms. Right-drag,
  middle-drag, or a left-drag on empty space pans.
- **Select (V):** click a node, path or region. Drag a node to move it, along with every loop node welded to it (shared nodes
  have a green ring). Drop a node onto another loop's node to share it, which is how a region is attached to the outline.
  Hold Alt to drag without snapping. Double-click a line to add a node: on an edge two loops share, the node goes into both.
  Delete removes the node, region or path. S makes a corner sharp or smooth again. PgUp/PgDn (or `]`/`[`) raise or sink the
  selected region by 20 (Shift: 100), or a path node's height.
- **Region (R):** click points, then click the first point or press Enter to close. A click on an existing node shares it.
  A click on a loop's edge adds a node there and shares it, so a region can be drawn against the outline or another
  region. New regions start 120 above the ground they're drawn on.
- **Path (P):** click points, then double-click or press Enter to finish. Put each end *inside* the floor it starts or
  finishes on, not on its edge: an end takes the floor's height and the builder lands it at the floor's edge (see the
  paths section of `../overworld/README.md`). An end exactly on an edge could take either side's height, so path clicks never snap
  to region nodes. The panel sets width, mode per segment (attached embankment or floating bridge), side style and
  bridge shape. A path node's height and width are optional, per node.
- **Brush (B):** paints the level's terrain (see Painted terrain in `../overworld/README.md`): one smooth height offset that
  everything rides on, so painting across several regions raises them all together, with their walls, paths and bridges.
  Modes: Raise, Lower, Smooth, Flatten, Bumps, Erase (keys 1-6). Ctrl turns raise into lower; Shift smooths. `[` and `]`
  size it; *Hard core* is the share of the radius at full strength. Paint in the plan or in 3D (there, Alt-drag or
  middle-drag orbits). Each stroke is one undo step. The rebuild follows a few times a second while you paint.
- **Several regions:** Shift-click regions (in the plan, in 3D or in Contents) to select several. PgUp/PgDn, the panel's
  -20/+20 and dragging in 3D raise or sink them all together; Delete deletes them all.
- **Bumps:** a region's panel (and the outline's) has *Bumps*: height, size, edge fade and seed. See the Bumps section
  of `../overworld/README.md`. The Heights view shows them best from above, and the 3D view or Play up close.
- **Path profile:** selecting a path opens its side view under the plan, laid out by the builder's own `paths::layout`.
  You see the ground under it, embankments filled down to the ground, bridges in blue, and each segment's slope, with
  anything steeper than the theme's walkable slope (35°) in red. Drag a node up or down to set its height (5-unit
  steps; Shift: 1). Right-click it for "Automatic height". ×1 shows true slopes; ×2 and ×4 exaggerate the heights. The
  pointer's spot along the path is marked on the plan and in 3D.
- **3D view** (Plan + 3D, or 3D, in the top bar): the build in perspective, textured with its baked lighting as the game
  draws it. Drag to orbit, right-drag to pan, wheel to zoom; a double-click orbits round that spot. Click a region or
  path to select it, then **drag the selected region up or down to raise or sink it** (5-unit steps; Shift: 1). The
  selection is outlined at its height straight away, and the level catches up when the rebuild lands. F frames both
  views.
- **Play:** ▶ Play exports the build and starts the game (`oot_sandbox --level <out dir> --child`) with child Link: the USB
  N64 pad, or the keyboard (WASD, Space A, E B, Q Z-target). W plays from the cursor, and so does the right-click menu's
  "Play from here", facing north. While the game is open, every rebuild is exported and it reloads the level with Link
  where he stands. The level has Kokiri Forest's light, its surface types (grass, no-grab cliffs, vines) and a water
  box for each pond. A level whose collision needs more than 8192 vertices is refused: build it at Medium or Low.
- Undo/redo: Ctrl+Z / Ctrl+Y. Each drag or each field edit is one step. Save writes the document with each node on one line.

**Detail:** Level settings has High / Medium / Low (see Detail in `../overworld/README.md`); the Build panel's *Triangles* lists
each object's count.

The side panel has the selection's properties, a list of everything in the level, the build's status and problems (a
failed build shows the last good one and rings the spot the error names, such as loops crossing), the edge-of-the-world
and sampling settings, and Files (theme, texture library, export folder).

## Code

- `edit.rs` (no UI, unit tested): welded node groups, moving, inserting on shared edges, deleting, picking, the curves the
  builder draws (`overworld::map::sample_loops`, `overworld::paths::centre_line`), and the JSON writer.
- `worker.rs`: builds on a background thread, skipping documents that are already stale, and exports with
  `overworld::export::write`.
- `scene.rs`: the plan camera and the level drawn from above (egui meshes, lowest triangles first), plus floor heights
  under the cursor and ray casts against the floors (3D picking).
- `profile.rs`: a path's side profile, with the painted terrain included (node drags convert back to design heights).
- `view3d.rs` + `view3d.wgsl`: the 3D view, using the window's wgpu device (via `eframe::egui_wgpu`) to render offscreen
  with 4x MSAA and mipmapped textures, shown as an egui image, the way the OoT Clone's `oot_viewer` does it.
- `app.rs`: the window, tools, panels, undo, files and Play.

Not yet: editing nodes in 3D, props.
