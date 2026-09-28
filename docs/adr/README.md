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
