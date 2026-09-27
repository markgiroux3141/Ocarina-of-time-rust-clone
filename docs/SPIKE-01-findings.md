# Spike 01: Link models, materials and animations

**Question:** can we decode OoT's character models, materials and animations from the ROM, using the decomp as a map, well enough to build a Rust clone on?

**Answer:** yes. The formats decode completely and consistently, and the renderer reproduces N64 material behaviour.

ROM used: `D:/OOT Modding/Debug Roms/baserom.z64` (MD5 matches the decomp's `checksum.md5`, gc-eu-mq-dbg). Decomp: `D:/OOT Modding/OTT decomp/z64oot`.

## How it was tested

Two independent checks:

1. **Real ROM data, validated numerically** (`ootx`). Skeletons, display lists and animations from your ROM go through the decoders, and the results are checked for structural consistency and physical plausibility.
2. **Rendering, validated visually on an original test character** ("Tock", `oot_core::synth`). It is encoded in the same binary formats the game uses (F3DEX2 DLs, CI4+TLUT / I4 / I8 / IA8 / RGBA16 textures, flex LOD skeleton with segment-0x0D seam matrices, compressed `AnimationHeader` animations), then decoded and drawn by the same code.

Spike 01 validated the Link data numerically only. Rendering Link from the ROM was added afterwards; see [SPIKE-02-link-viewer.md](SPIKE-02-link-viewer.md).

## Results on the ROM

### Filesystem
| Check | Result |
|---|---|
| dmadata located by signature | 0x12F70, 1532 files, none compressed (debug ROM) |
| File names | Mapped from decomp `extract_baserom.py`. `spec` has one extra non-ROM segment (`buffers`) and is used only as a fallback |
| Decomp XML index | 1072 files: 194 skeletons, 1156 animations, 573 player animations |

### Link skeletons and display lists
| | Adult (`object_link_boy`) | Child (`object_link_child`) |
|---|---|---|
| Limbs | 21 (flex, LOD) | 21 (flex, LOD) |
| Flex matrix map vs header `dListCount` | 18 = 18 | 18 = 18 |
| Triangles (near LOD) | 735 | 697 |
| Distinct textures / materials | 32 / 34 | 18 / 20 |
| Unknown opcodes | 0 | 0 |
| Unresolved references | only segments 08/09 (eye/mouth textures bound by Player at runtime) | same |

The matrix-map match means the model of how limb DLs borrow neighbouring limbs' matrices (depth-first order, one matrix per limb with a DL) is right.

### Player animations (gameplay_keep headers → link_animetion data)
| Check | Result |
|---|---|
| Parsed | 573 / 573 |
| Frame counts matching decomp XML | 573 / 573 (18,733 frames) |
| Frame layout | 22 × Vec3s + u16 face = 134 bytes/frame, confirmed from XML offsets |
| Standing idle (`link_normal_wait`), adult | lowest vertex stays at y −29…−6 across 89 frames; max limb step 1.8°/frame |
| Walk, adult | lowest vertex −120…9 (feet planting) |
| Child with the game's 0.64 root scale | idle lowest vertex −74…−49, so it's grounded |

Feet staying on the ground through the idle is a strong check of the rotation convention (translate, then Rz·Ry·Rx, binary angles). A wrong Euler order would make the feet drift. The raw per-component angle deltas look alarming (p90 ≈ 180°), but those are Euler-angle flips: the true rotation between frames has p50 29.5° and p99 99°.

Child Link has no animations of its own for most moves. It plays the adult data with the root translation scaled by 0.64 (`Player_OverrideLimbDrawGameplayCommon`), and the numbers above confirm that.

### Whole game (`ootx scan-skeletons`)
| Check | Result |
|---|---|
| Skeletons parsed + all limb DLs interpreted | 184 / 194 |
| Unknown GBI opcodes, all objects | 0 (≈50,800 triangles) |
| Standard actor animations parsed | 1392 / 1393 |
| Fully self-contained (no runtime segments needed) | 75 |
| Not yet supported | 7 Skin skeletons (horses/Epona), 3 Curve skeletons (chest lid, warp effects) |

Unresolved references are almost all segments 08–0B, which actors bind at runtime (eye textures, colour variants). Each actor's draw code determines them, so they'll need a per-actor table, which the decomp source provides.

## Results on the test character (rendering)

`out/tock_sheet.png` (contact sheet) and `out/tock_walk.png` / `out/tock_wave.png`. Everything the character uses renders as intended:

- CI4 texture with a 16-entry RGBA16 TLUT
- I4 emblem tinted by env colour through the combiner (`(ENV − SHADE) × TEXEL0 + SHADE`, the same trick as tunic colours)
- 2-cycle combiner (lit texel, then × PRIM) with mirror wrap
- RGBA16 face fetched through segment 0x08 and swapped per frame (blink / happy), the same mechanism as Link's eyes
- IA8 alpha cut-out (`G_RM_AA_ZB_TEX_EDGE`) and a translucent bulb (`G_RM_AA_ZB_XLU_SURF`)
- Unlit vertex colours vs lit normals
- Flex seams at neck, elbows and knees stretching correctly across bent joints
- Compressed animations with static/dynamic tracks, and a full 360° spin interpolated across binary-angle wrap-around

A bug surfaced visually that the numeric checks couldn't catch: triangle indices were halved twice, which scrambled faces while leaving triangle counts and vertex bounds unchanged. It's fixed, and the ROM validation was re-run afterwards.

## Known simplifications

- **TMEM:** modelled as a flat byte array without odd-row swizzling. That's equivalent for well-formed `LoadTextureBlock`/`LoadTile` loads, which is what the game uses.
- **Lighting:** a fixed viewer light. Scene/actor lights (`G_MOVEMEM` light structs) are ignored, and so is fog.
- **Combiner edge cases:** noise, key, LOD fraction and K4/K5 evaluate to 0. There's no N64 half-texel offset or 3-point filtering; the renderer uses GPU bilinear.
- **`G_BRANCH_Z`:** always takes the near branch.
- **Four `object_oE*` skeletons:** the XML labels them "Normal", but their DLs load segment-0x0D matrices. These are unused debug objects, so this is still open.

## Recommended next spikes

1. ~~**Player draw rules**~~: done in spike 02 (hands, sheath, shield, waist, face, tunic). Masks, boots and gauntlets are still open.
2. **Scenes and rooms:** room mesh headers, collision, and a static render of a scene. The same interpreter applies, plus lights and fog.
3. **Skin and Curve limbs:** covers horses and the chest lid, finishing skeleton coverage.
4. **Game logic:** port one simple actor's update function from decomp C to Rust against a minimal actor/collision framework. That's the real feasibility question for "logic from the decomp".
5. **Retail ROM support:** Yaz0 is implemented but untested. Try a compressed NTSC 1.0 ROM once a decomp XML set for it is available.
