# 0004: Decomp commit: stay on `2f4c25d` through the asset pack

- **Status:** accepted; to be superseded by Phase 6's first milestone, the upgrade (ADR 0028)
- **Date:** 2026-09-27

## Context

The decomp checkout at `D:/OOT Modding/OTT decomp/z64oot` is at commit `2f4c25da53b3a23251a6f2ab477d588b4da38475` (2022-10-04, "Fix many missing NULLs (#1389)").
- Its functions are mostly still `func_808xxxxx`, and many fields are `unk_XXX`.
- Assets are extracted from the ROM using the XML offsets. There is no multi-version support.

Every port in the repo cites these names: comments, test expectations, and the `csrc` and `drawcfg` readers that parse its C.

Later commits name most of Player (`Player_Action_*`) and the camera, and support several ROM versions. Upgrading would help readability and ADR 0003's vanilla-dungeon question. But it would rename most citations, and change the C that `csrc` and `drawcfg` parse, while the code is being restructured.

## Decision

- Pin `2f4c25d` through milestone 2 (the asset pack). The import validation, the table extraction and the oracle tests are all written against it.
- Re-evaluate after milestone 2, when the runtime no longer parses C and only the importer and the oracles would be affected.

An upgrade then comes with:
- a name map (old `func_`/`D_`/`unk_` name → new name), generated from the two commits' symbol files, and applied to comments and docs by a script;
- `csrc`/`drawcfg` fixes, proven by the same oracle tests passing on the new commit;
- the golden hashes and all tests unchanged.

## Consequences

- `oot.toml`'s `decomp` path must point at this commit. The importer should record the decomp commit it read in the pack header (milestone 2), so a pack built from another commit is identifiable.
- New ports keep citing the old names. When a newer name is known and helps, it can be added in the comment (`func_80834A2C` (`Player_UpperAction_ChangeHeldItem`)), but the old name stays first until the upgrade.
