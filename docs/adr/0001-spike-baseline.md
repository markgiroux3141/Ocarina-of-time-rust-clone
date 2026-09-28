# 0001: The spikes are committed and tagged, and their renders are hashed

- **Status:** accepted
- **Date:** 2026-09-27

## Context

Four spikes proved the approach (decode the ROM with the decomp's help, port the logic function by function). The restructure that follows moves almost every file. Moving files over uncommitted work loses history, and a restructure that claims "no behaviour change" needs something to compare against.

## Decision

- The spike state is commit `aea9455` ("Spike 04: Kokiri Forest from the ROM, camera, Player layers, water, DynaPoly") on `master`, tagged `spikes-complete` (a lightweight local tag; there is no remote).
- Files are moved with `git mv`, so `git log --follow` crosses the restructure.
- The spikes' behaviour is pinned by two suites:
  - The 87 tests (`cargo test --workspace`), whose expected values come from the decomp's C.
  - `scripts/golden.py`: 54 headless cases (every sandbox script as a contact sheet and a JSON trace, screenshots of all 12 Kokiri Forest spawns, other times of day, the collision view, Hyrule Field, the Deku Tree, and the viewer's Link and Tock sheets). The images are hashed on their decoded RGBA pixels and the traces on their bytes. The 78 hashes are committed in `golden/renders.sha256`; the images stay in the git-ignored `out/golden/`.
- The hashes were recorded from the spike binaries (`oot_play`, `oot_viewer`) at `spikes-complete`, and re-rendering twice gave identical hashes, so the renders are deterministic on this machine.

## Consequences

- Every later milestone runs `python scripts/golden.py check`. A hash that changes is either a regression or a deliberate change, and a deliberate change re-records the hashes with the reason in that milestone's notes.
- The hashes are specific to this machine's GPU and driver. On another machine, record a baseline at `spikes-complete` first.
- No game data is committed: only the hashes of renders made locally from the user's ROM.
