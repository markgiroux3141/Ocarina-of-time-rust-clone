//! Thumbnails of the kit's pieces for the Kit panel: each piece drawn from the front and a little
//! to the side, from above, textured and lit (`overworld::thumb`'s software rasteriser, so they
//! need no GPU readback and come out the same everywhere). Made once per piece, when first shown.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use overworld::pieces::Piece;
use overworld::textures::Library;
use overworld::thumb::{self, Texels};
use std::collections::HashMap;

/// The thumbnail's side, in pixels.
pub const SIZE: usize = 112;

#[derive(Default)]
pub struct Thumbs {
    made: HashMap<String, Option<TextureHandle>>,
    texels: Texels,
}

impl Thumbs {
    /// The piece's thumbnail, made now if it hasn't been.
    pub fn get(&mut self, ctx: &egui::Context, piece: &Piece, lib: Option<&Library>) -> Option<egui::TextureId> {
        if !self.made.contains_key(&piece.name) {
            let img = thumb::render(piece, lib, &mut self.texels, SIZE).map(|i| ColorImage::from_rgba_unmultiplied([i.w, i.h], &i.px));
            let h = img.map(|img| ctx.load_texture(format!("thumb:{}", piece.name), img, TextureOptions::LINEAR));
            self.made.insert(piece.name.clone(), h);
        }
        self.made[&piece.name].as_ref().map(|h| h.id())
    }
}
