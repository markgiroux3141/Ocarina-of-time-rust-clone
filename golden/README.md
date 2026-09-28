# Golden hashes

`renders.sha256` pins the spikes' behaviour: every headless case of `scripts/golden.py`, a
PNG hashed on its decoded pixels or a JSON trace hashed on its bytes (docs/adr/0001). The
images themselves stay in the git-ignored `out/golden/`. The hashes are specific to one
machine's GPU and driver.

A hash only changes on purpose. Each re-recording is listed here with its reason and what it
changed.

| When | Why | Changed |
|---|---|---|
| 2026-09-27, `spikes-complete` | First recording, from the spike binaries (`oot_play`, `oot_viewer`) | 78 hashes, 54 cases |
| 2026-09-27, GAME-01 milestone 2 | none: the asset pack renders identically | nothing |
| 2026-09-27, GAME-01 milestone 3 | Draw submission (OPA list, then XLU list, as `Play_Draw`). On the course, the spike drew Link's circle shadow before Link; it's in the XLU list now, after every opaque draw | the 24 course images and the scene collision view: 9 to 3,263 pixels each, the soles of Link's boots where the translucent shadow now blends over them. All traces and all room renders unchanged |
| 2026-09-27, GAME-01 milestone 4 | none: the sandbox's scenes stay the spikes' view unless `--entrance` is given, and the draw configs now run once per game frame in `PlayState` with the same values | nothing |
