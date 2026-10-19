//! The game's text box in the inspector (docs/EDITOR-ACTORS.md, "Texts"): a line typed out by the
//! game's own message box (`oot_game::message_text::preview`, the real `MessageContext`), and its
//! sprites drawn by the game's renderer (`eng_render`) from the pack's bakes (the box's image, the
//! font's glyphs, the icons), as the game draws them over the 320x240 screen. Each box is one
//! picture, cropped to the box, made the first time it's asked for and kept.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use eng_gfx::{DrawList, DrawLists, MeshKey};
use oot_game::message::MessageTable;
use oot_game::message_text::{BoxEnd, PreviewBox, TextOverride};
use oot_game::pack::GamePack;

/// The screen drawn at twice the game's 320x240, for the inspector's crisp boxes.
const SCALE: u32 = 2;
/// What's behind the box: the game draws it over the scene; a dim green, as a forest's shade.
const BACKDROP: [f64; 4] = [0.20, 0.24, 0.21, 1.0];
/// Boxes made per frame (the rest come next frame).
const PER_FRAME: usize = 4;

/// The pack's meshes for the renderer: the message box's sprites are bakes (`bake/message/...`).
struct PackMeshes<'a>(&'a GamePack);

impl eng_render::MeshSource for PackMeshes<'_> {
    fn mesh(&mut self, key: &MeshKey) -> Option<DrawList> {
        self.0.assets.try_get::<DrawList>(&key.name).ok().flatten()
    }
}

/// One box drawn: its picture, the part of it that's the box (in UV), its size in screen pixels.
#[derive(Clone, Copy)]
pub struct Drawn {
    pub texture: egui::TextureId,
    pub uv: egui::Rect,
    pub size: egui::Vec2,
    pub end: BoxEnd,
    pub text_id: u16,
}

pub struct TextBoxes {
    rs: egui_wgpu::RenderState,
    renderer: eng_render::Renderer,
    cache: eng_render::MeshCache,
    target: Option<eng_render::Target>,
    pack: Option<GamePack>,
    /// The game's messages.
    pub table: Option<Arc<MessageTable>>,
    /// Why there are none (no pack).
    pub error: Option<String>,
    /// Boxes by (text id, its bytes and box, the box's index).
    made: HashMap<u64, Vec<Drawn>>,
    textures: Vec<(egui::TextureId, wgpu::Texture)>,
    budget: usize,
    /// The game's messages with the level's texts over them, and the texts' hash they're from.
    level: Option<(u64, Arc<MessageTable>)>,
}

impl TextBoxes {
    pub fn new(rs: &egui_wgpu::RenderState) -> TextBoxes {
        let (pack, table, error) = match GamePack::open_default().and_then(|p| Ok((p.messages()?, p))) {
            Ok((t, p)) => (Some(p), Some(Arc::new(t)), None),
            Err(e) => (None, None, Some(format!("the game's pack: {e:#}"))),
        };
        TextBoxes {
            rs: rs.clone(),
            renderer: eng_render::Renderer::new(&rs.device, &rs.queue),
            cache: eng_render::MeshCache::new(),
            target: None,
            pack,
            table,
            error,
            made: HashMap::new(),
            textures: vec![],
            budget: PER_FRAME,
            level: None,
        }
    }

    /// A new frame: the boxes' budget again.
    pub fn new_frame(&mut self) {
        self.budget = PER_FRAME;
    }

    /// The boxes `text_id` shows in `table` (the game's with the level's texts over it), up to
    /// `max`, drawn; `None` until it's made (a few a frame). `key` tells the texts apart (the
    /// level's text and box, or the game's).
    pub fn boxes(&mut self, table: &MessageTable, text_id: u16, key: u64, max: usize) -> Option<Vec<Drawn>> {
        if let Some(d) = self.made.get(&key) {
            return Some(d.clone());
        }
        if self.budget == 0 {
            return None;
        }
        self.budget -= 1;
        let boxes = oot_game::message_text::preview(table, text_id, "Link", max);
        let drawn: Vec<Drawn> = boxes.iter().filter_map(|b| self.draw(b)).collect();
        // a bounded keep: the pictures of lines not looked at in a while go
        if self.made.len() > 200 {
            self.clear();
        }
        self.made.insert(key, drawn.clone());
        Some(drawn)
    }

    /// The messages as the level plays them: the game's, with the level's texts that encode over
    /// them (`MessageTable::layered`, as `Play_Init` lays them out). Made again when they change.
    pub fn level_table(&mut self, texts: &[overworld::doc::Text]) -> Option<Arc<MessageTable>> {
        let game = self.table.clone()?;
        let h = texts_hash(texts);
        if let Some((k, t)) = &self.level
            && *k == h
        {
            return Some(t.clone());
        }
        let over: Vec<TextOverride> = texts
            .iter()
            .filter_map(|t| Some(TextOverride { text_id: t.id, type_pos: (t.box_type << 4) | (t.box_pos & 0xF), bytes: oot_game::message_text::encode(&t.text).ok()? }))
            .collect();
        let t = if over.is_empty() { game } else { Arc::new(game.layered(&over)) };
        self.level = Some((h, t.clone()));
        Some(t)
    }

    /// Drops every picture (after the pack changes, or too many).
    pub fn clear(&mut self) {
        self.made.clear();
        let mut er = self.rs.renderer.write();
        for (id, _) in self.textures.drain(..) {
            er.free_texture(&id);
        }
    }

    fn draw(&mut self, b: &PreviewBox) -> Option<Drawn> {
        let pack = self.pack.as_ref()?;
        let device = self.rs.device.clone();
        let queue = self.rs.queue.clone();
        let (w, h) = (320 * SCALE, 240 * SCALE);
        let target = self.target.get_or_insert_with(|| eng_render::Target::new(&device, w, h));
        let lists = DrawLists { overlay_2d: b.sprites.iter().map(|s| s.draw_cmd()).collect(), ..Default::default() };
        let cam = eng_render::Camera::framing(glam::Vec3::ZERO, glam::Vec3::ONE, 0.0, 0.0, 1.0);
        let mut enc = device.create_command_encoder(&Default::default());
        self.renderer.render_lists(&device, &queue, &mut enc, target, &lists, &mut self.cache, &mut PackMeshes(pack), &cam, &eng_render::Lighting::default(), &[], BACKDROP);
        let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("text box"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: eng_render::COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        enc.copy_texture_to_texture(target.resolve.as_image_copy(), tex.as_image_copy(), size);
        queue.submit([enc.finish()]);
        let id = self.rs.renderer.write().register_native_texture(&device, &tex.create_view(&Default::default()), wgpu::FilterMode::Nearest);
        self.textures.push((id, tex));
        // the box and a margin round it: 8 above, 6 to the sides and below (the shadows, the arrow)
        let [x, y, bw, bh] = b.rect.map(|v| v as f32);
        let (x0, y0, x1, y1) = ((x - 6.0).max(0.0), (y - 8.0).max(0.0), (x + bw + 6.0).min(320.0), (y + bh + 6.0).min(240.0));
        let uv = egui::Rect::from_min_max(egui::pos2(x0 / 320.0, y0 / 240.0), egui::pos2(x1 / 320.0, y1 / 240.0));
        Some(Drawn { texture: id, uv, size: egui::vec2(x1 - x0, y1 - y0), end: b.end, text_id: b.text_id })
    }
}

/// A hash of the level's texts (what tells one set from another).
fn texts_hash(texts: &[overworld::doc::Text]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for t in texts {
        (t.id, t.box_type, t.box_pos, &t.text).hash(&mut h);
    }
    h.finish()
}

/// A hash of what a line's boxes are drawn from: each text's box and bytes along its chain.
pub fn boxes_key(table: &MessageTable, chain: &[u16]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for &id in chain {
        (id, table.entry(id).map(|e| e.type_pos), table.raw(id)).hash(&mut h);
    }
    h.finish()
}
