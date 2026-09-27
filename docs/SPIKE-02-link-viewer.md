# Spike 02: Link in the viewer

**Question:** does Link from the ROM, driven by his real animations, look right when put through the spike 01 pipeline?

**Answer:** yes. Adult and child Link render correctly in every pose checked: idle, walk, run, jump, sword slashes and shield guard. No renderer or decoder changes were needed. The only missing piece was Player's runtime draw state, which is now read from the decomp.

## What was added

- **`oot_core::csrc`**: a small parser for static C initializers (`type name[..] = { ... };`, nested braces, comments) and `/* 0xNN */` enum members. Where `#ifndef AVOID_UB` has two variants, the first definition (the as-shipped one) wins.
- **`oot_core::player`**: Player's draw rules, loaded from `src/code/z_player_lib.c` and `include/z64player.h` at runtime:
  - `sPlayerDListGroups` and every table it names, plus `gPlayerModelTypes` (16 model groups → left hand, right hand, sheath, waist)
  - `Player_OverrideLimbDrawGameplayDefault` logic: shield-indexed right-hand and sheath variants, open hands turning into fists when `speedXZ > 2`, and child without the Kokiri Sword getting the swordless sheath
  - `Player_DrawImpl`: the eye texture on segment 0x08 and mouth on 0x09 (animation face field first, then `sEyeMouthIndices` from the blink state), and the tunic colour as env colour
  - the blink timer (`func_80032CB4(…, 20, 80, 6)`)
  - the child 0.64 root-translation scale
- **`BuildOptions::limb_dlists`**: per-limb DL overrides (the `OverrideLimbDraw` equivalent). The segment-0x0D matrix map now counts a limb when either its own DL or the override is non-NULL, matching `SkelAnime_DrawFlexLimbLod`.
- **Viewer**: Link is the default subject (Tock is still available). It has the full list of 573 Player animations with a filter, plus age, model group, shield, tunic, LOD and eye/mouth controls.
- **`ootx player-draw`**: builds every age × model group × shield combination.

## Results

| Check | Result |
|---|---|
| Rules read from C | 21 model types, 16 model groups, 4 shields, 3 tunics, 8 eye / 4 mouth textures |
| Loadouts built (2 ages × 16 groups × 4 shields) | 128 |
| DL names missing from the object XMLs | 0 |
| Unknown opcodes / unresolved segment refs | 0 / 0 (segments 08/09 are now bound) |
| Triangles per loadout | 661 … 813 |

Visual checks, from headless renders written to `out/` (git-ignored):

- Adult DEFAULT/Hylian: Hylian shield and Master Sword on the back, tunic green from env colour, CI8 eyes and mouth through segments 08/09
- Child DEFAULT/Deku: Deku shield and Kokiri Sword on the back. With the 0.64 root scale the feet stay on the ground in walk and run
- SWORD group: sword in the left hand and shield in the right, through the slash, finisher and guard animations
- Joint seams at the knees, elbows and neck hold together in extreme poses (lunges, the jump)

## Still open

- Boots, gauntlets and masks. These are drawn after the skeleton in `Player_DrawImpl` and `Player_PostLimbDrawGameplay`, so they need post-limb DLs attached to a limb matrix.
- Items drawn separately (bottle contents, bow string, hookshot chain, Deku stick).
- The head/upper-body look-at rotations (`unk_6B*`) and foot IK (`func_8008F87C`) are gameplay-driven, so they're zero here.
- Root motion: animations play in place, with the root translation shown raw. Player's `moveFlags` handling isn't modelled.
- Lighting is still the fixed viewer light.
