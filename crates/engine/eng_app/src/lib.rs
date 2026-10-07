//! App shell: the pieces every windowed or headless app of the engine shares.
//!
//! - `run_window`: opens an eframe window on the wgpu backend.
//! - `ViewportTarget`: an offscreen `eng_render::Target` sized to the panel and shown as an
//!   egui image (registered with egui's renderer once, then updated on resize).
//! - `keyboard`: the default keyboard layout for the N64 pad (`eng_input::device::Keyboard`).
//! - `save_png`: writes a readback, creating the folder.
//!
//! The fixed-step game loop with interpolated rendering lives with the game state for now
//! (`oot_actors::world::World::advance`); it moves here when PlayState owns the frame.

use std::path::Path;

pub use eframe;
pub use egui_wgpu;
use anyhow::Result;
use eframe::egui;
use eng_input::device::Keyboard;
use eng_render::Target;

/// Opens a window of `size` pixels titled `title` and runs `app` in it until it's closed.
pub fn run_window<A: eframe::App + 'static>(
    name: &str,
    title: &str,
    size: [f32; 2],
    app: impl FnOnce(&eframe::CreationContext) -> Result<A> + 'static,
) -> Result<()> {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(size).with_title(title),
        ..Default::default()
    };
    // The renderer's optional features (`eng_render::NON_FEATURES`: the game's NoN
    // microcode), where the adapter has them.
    if let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup {
        let base = setup.device_descriptor.clone();
        setup.device_descriptor = std::sync::Arc::new(move |adapter| {
            let mut d = base(adapter);
            d.required_features |= adapter.features() & eng_render::NON_FEATURES;
            d
        });
    }
    eframe::run_native(name, options, Box::new(move |cc| Ok(Box::new(app(cc)?)))).map_err(|e| anyhow::anyhow!("{e}"))
}

/// The wgpu device and queue of the window, and egui's renderer.
pub fn wgpu_state(cc: &eframe::CreationContext) -> Result<(wgpu::Device, wgpu::Queue, egui_wgpu::RenderState)> {
    let rs = cc.wgpu_render_state.clone().ok_or_else(|| anyhow::anyhow!("wgpu backend required"))?;
    Ok((rs.device.clone(), rs.queue.clone(), rs))
}

/// An offscreen render target shown in the UI.
#[derive(Default)]
pub struct ViewportTarget {
    inner: Option<(Target, egui::TextureId)>,
}

impl ViewportTarget {
    /// Makes the target `width` x `height` pixels (at least 1x1): created on first use,
    /// recreated when the size changes. Returns its egui texture.
    pub fn ensure(&mut self, device: &wgpu::Device, render_state: &egui_wgpu::RenderState, width: u32, height: u32) -> egui::TextureId {
        if self.inner.as_ref().is_none_or(|(t, _)| t.size != (width.max(1), height.max(1))) {
            let t = Target::new(device, width, height);
            let mut er = render_state.renderer.write();
            let id = match self.inner.take() {
                Some((_, id)) => {
                    er.update_egui_texture_from_wgpu_texture(device, &t.resolve_view, wgpu::FilterMode::Linear, id);
                    id
                }
                None => er.register_native_texture(device, &t.resolve_view, wgpu::FilterMode::Linear),
            };
            self.inner = Some((t, id));
        }
        self.inner.as_ref().unwrap().1
    }

    /// The target, once `ensure` has made it.
    pub fn target(&self) -> &Target {
        &self.inner.as_ref().expect("ViewportTarget::ensure first").0
    }
}

/// The keyboard as an N64 pad: WASD/arrows stick, Shift walk, Space A, E B, Q Z, R R, T L,
/// Enter Start, J/L/I/K the C buttons.
pub fn keyboard(ctx: &egui::Context) -> Keyboard {
    use egui::Key::*;
    ctx.input(|i| {
        let k = |keys: &[egui::Key]| keys.iter().any(|&x| i.key_down(x));
        Keyboard {
            up: k(&[W, ArrowUp]),
            down: k(&[S, ArrowDown]),
            left: k(&[A, ArrowLeft]),
            right: k(&[D, ArrowRight]),
            walk: i.modifiers.shift,
            a: k(&[Space]),
            b: k(&[E]),
            z: k(&[Q]),
            r: k(&[R]),
            l: k(&[T]),
            start: k(&[Enter]),
            c_left: k(&[J]),
            c_right: k(&[L]),
            c_up: k(&[I]),
            c_down: k(&[K]),
        }
    })
}

/// Writes tightly packed RGBA8 pixels as a PNG, creating the folder.
pub fn save_png(path: &Path, w: u32, h: u32, px: &[u8]) -> Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    image::save_buffer(path, px, w, h, image::ColorType::Rgba8)?;
    Ok(())
}
