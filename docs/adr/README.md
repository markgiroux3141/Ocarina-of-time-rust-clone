# Architecture decision records

One file per decision: the context, what was decided, and what follows from it. A decision that changes gets a new record that supersedes the old one; the old one stays, marked as superseded.

| # | Decision | Status |
|---|---|---|
| [0001](0001-spike-baseline.md) | The spikes are committed and tagged `spikes-complete`, and their renders are hashed | Accepted |
| [0002](0002-crate-layout.md) | Crate layout: engine, import, game, content, apps, tools; a test enforces the layering | Accepted |
| [0003](0003-rom-version.md) | Target ROM: gc-eu-mq-dbg for development | Accepted |
| [0004](0004-decomp-commit.md) | Decomp commit: stay on `2f4c25d` through the asset pack | Accepted |
| [0005](0005-engine-name.md) | Engine name: the `eng_` placeholder prefix | Accepted |
| [0006](0006-rendering-model.md) | Rendering model: hybrid (baked meshes plus a draw API, runtime display lists where needed) | Accepted; draw submission built |
| [0007](0007-actor-ownership.md) | Actor ownership: a generational arena, with actors taken out of their slot during their update | Accepted, built |
| [0008](0008-asset-pack.md) | Asset pack: one versioned file per ROM in the user data dir, keyed by decomp names | Accepted, built |
| [0009](0009-baked-state.md) | Baking what runtime state changes: meshes per scene layer, Link's face as a texture swap, draw configs ported | Accepted |
| [0010](0010-scenes-and-spawning.md) | Scenes and spawning: `Play_Init` rebuilds the play state, actor profiles from the C, placeholders for unported actors, loads keep their timing | Accepted, built |
| [0011](0011-collision-check.md) | The collision check: colliders stay in their actors; the context holds references, and the checks take them out and put them back | Accepted, built |
| [0012](0012-actor-bakes.md) | Actor bakes: actors declare their meshes (lists or a skeleton, segments, a prelude), the importer bakes them | Accepted, built |
| [0013](0013-camera-modes-and-screen.md) | Camera modes by the imported `CAM_FUNC`, unported modes on Normal1; the letterbox and an orthographic overlay list in the engine; the reticle at 20 Hz | Accepted, built |
| [0014](0014-prerendered-backgrounds.md) | Prerendered rooms: JPEG backgrounds decoded at import into screen quads drawn in the OPA list with the fixed camera; the room skyboxes baked from `Skybox_Init`'s display lists | Accepted, built |
| [0015](0015-camera-settings-and-bg-cameras.md) | Camera settings from `sCameraSettings`, the scene's bg cameras in the collision header, the setting changes ported, unported functions on the setting's NORMAL one | Accepted, built |
| [0016](0016-player-requests.md) | Player queues what it does to the camera, the rooms and other actors (`PlayRequest`), applied right after its update; actors write each other through the arena | Accepted, built |
| [0017](0017-interface-sprites.md) | The message box and the HUD as baked sprites: each texture with its setup baked once as a quad, drawn in `overlay_2d` with a transform and per-draw colours; projective transforms divided on the CPU | Accepted, built |
| [0018](0018-bg-treemouth.md) | The Deku Tree's mouth: its flag logic ported with the cutscene triggers logged against an idle `csCtx`; its env alpha as a dynamic colour on segment 0x0B; debug save presets for the missing cutscenes' flags; DynaPoly deletion by the C's slot flags | Accepted, built |
| [0019](0019-inventory-and-saves.md) | The inventory and saves: `SaveContext::new` is `Sram_InitNewSave`, `debug` the map select's `Sram_InitDebugSave`; the item tables from the C; `GetItem_Draw` baked as pieces with `Gfx_TwoTexScroll` as a dynamic segment; Link's variants keyed by their lists; a stand-in for the pause menu's equipping on Start | Accepted, built |
