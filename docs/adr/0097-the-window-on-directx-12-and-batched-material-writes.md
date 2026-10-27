# 0097: The window on DirectX 12, and batched material writes

- **Status:** accepted, built (2026-10-27)
- **Date:** 2026-10-27

## Context

The user's hand tests ran at about 15 fps on an RTX 3080, falling to 8 in busy scenes: from
230 fps at the start (the fade, nothing posed) to 15 within two seconds. Milestone 3's build
(eddf199) was the same: not a regression of this phase's.

Measured headless (the window's `draw_frame`, Dampé's grave, 1280x720): 55 ms a frame, of which the
game's update 0.1 ms and `Renderer::render_lists` about 60 ms. Within it, `GpuModel::set_draw_values`
wrote each dynamic, lit or fogged material's uniform with its own `queue.write_buffer`: Link's 54
materials about 24 ms, Dampé's 28 about 6 ms, every HUD quad about 0.5 ms. Each write cost about
0.45 ms on wgpu's default backend on Windows, Vulkan (the NVIDIA driver of 2025-02-24). On
DirectX 12 the same frame took 1.7 ms.

## Decision

1. **`GpuModel::set_draw_values` writes only what changed, once:** a CPU copy of the material
   buffer (`material_shadow`) is compared with each uniform the draw computes; the changed ones are
   copied in and written in one `write_buffer` over the range they span. The buffer's bytes are the
   same as before, so no image changes.
2. **The window (`eng_app::run_window`: the game, the sandbox's window, the tools) uses DirectX 12 on
   Windows** unless `WGPU_BACKEND` says otherwise. Headless rendering (`headless_device`: the
   goldens, the tests) keeps wgpu's default, so the golden hashes stand.

## Consequences

- Dampé's grave: Vulkan 55 ms to about 8 ms a frame (the rest is other per-frame writes, the
  skinned meshes' vertices); DirectX 12 1.7 ms to about 0.9 ms. The window runs well over 60 fps.
- The level editor's views, which share `eng_render` and `eng_app`, get both.
- `WGPU_BACKEND=vulkan` still picks Vulkan for a comparison.
