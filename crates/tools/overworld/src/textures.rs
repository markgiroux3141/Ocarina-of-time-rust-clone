//! A texture library: a folder of PNGs plus `textures.json` saying how each is drawn (wrap per
//! axis, alpha, culling). `overworld kit-textures` (`kit.rs`) writes the Kokiri one from the
//! clone's extracted scene.
//! The builder copies the textures a level uses next to its OBJ and writes the MTL from this.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TexInfo {
    pub file: String,
    #[serde(default)]
    pub size: Vec<u32>,
    /// "repeat", "clamp" or "mirror".
    #[serde(default = "repeat")]
    pub wrap_u: String,
    #[serde(default = "repeat")]
    pub wrap_v: String,
    /// "opaque", "cutout" (alpha tested) or "blend" (translucent, at `opacity`).
    #[serde(default = "opaque")]
    pub alpha: String,
    #[serde(default = "one")]
    pub opacity: f64,
    /// "back" or "none" (double-sided).
    #[serde(default = "back")]
    pub cull: String,
}

fn repeat() -> String {
    "repeat".into()
}
fn opaque() -> String {
    "opaque".into()
}
fn back() -> String {
    "back".into()
}
fn one() -> f64 {
    1.0
}

impl TexInfo {
    pub fn plain(name: &str) -> TexInfo {
        TexInfo { file: format!("{name}.png"), size: vec![], wrap_u: repeat(), wrap_v: repeat(), alpha: opaque(), opacity: 1.0, cull: back() }
    }

    /// Flags in the GE64 OBJ dialect's material names, which pd-walk reads: per-axis clamp and
    /// mirror, Cutout (alpha-tested, drawn with the opaque geometry so it sorts by depth: tree
    /// cards, roots), Transparent (alpha-blended: water), CullBoth (double-sided).
    pub fn flags(&self) -> Vec<&'static str> {
        let mut f = vec![];
        match self.wrap_u.as_str() {
            "clamp" => f.push("ClampS"),
            "mirror" => f.push("MirrorS"),
            _ => {}
        }
        match self.wrap_v.as_str() {
            "clamp" => f.push("ClampT"),
            "mirror" => f.push("MirrorT"),
            _ => {}
        }
        match self.alpha.as_str() {
            "cutout" => f.push("Cutout"),
            "blend" => f.push("Transparent"),
            _ => {}
        }
        if self.cull == "none" {
            f.push("CullBoth");
        }
        f
    }
}

/// A texture made of rows `r0..r1` (from the top) of texture `base`, mirror-repeating
/// vertically if `mirror`: a capped wall's middle, so one band can repeat it as often as needed.
pub fn derived_name(base: &str, r0: u32, r1: u32, mirror: bool) -> String {
    format!("{base}@{r0}-{r1}{}", if mirror { "m" } else { "" })
}

fn parse_derived(name: &str) -> Option<(&str, u32, u32, bool)> {
    let (base, rest) = name.rsplit_once('@')?;
    let mirror = rest.ends_with('m');
    let (a, b) = rest.trim_end_matches('m').split_once('-')?;
    Some((base, a.parse().ok()?, b.parse().ok()?, mirror))
}

/// RGBA8 pixels from PNG bytes, row 0 at the top.
pub fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px.chunks(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some((w, h, rgba))
}

pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().expect("png header");
        wr.write_image_data(rgba).expect("png data");
    }
    out
}

pub struct Library {
    pub dir: PathBuf,
    pub info: BTreeMap<String, TexInfo>,
}

impl Library {
    /// `dir/textures.json` if there is one; otherwise every texture is `<name>.png`, plain.
    pub fn load(dir: &Path) -> Result<Library, String> {
        let meta = dir.join("textures.json");
        let info = if meta.exists() {
            let s = std::fs::read_to_string(&meta).map_err(|e| format!("{}: {e}", meta.display()))?;
            serde_json::from_str(&s).map_err(|e| format!("{}: {e}", meta.display()))?
        } else {
            BTreeMap::new()
        };
        Ok(Library { dir: dir.to_path_buf(), info })
    }

    pub fn get(&self, name: &str) -> TexInfo {
        if let Some((base, r0, r1, mirror)) = parse_derived(name) {
            let b = self.get(base);
            let stem = b.file.strip_suffix(".png").unwrap_or(&b.file).to_string();
            return TexInfo {
                file: format!("{stem}-rows{r0}-{r1}.png"),
                size: b.size.first().map(|&w| vec![w, r1.saturating_sub(r0)]).unwrap_or_default(),
                wrap_v: if mirror { "mirror".into() } else { "repeat".into() },
                ..b
            };
        }
        self.info.get(name).cloned().unwrap_or_else(|| TexInfo::plain(name))
    }

    /// A texture's pixels (RGBA8, row 0 at the top), derived ones cut from their base.
    pub fn rgba(&self, name: &str) -> Option<(u32, u32, Vec<u8>)> {
        if let Some((base, r0, r1, _)) = parse_derived(name) {
            let (w, h, px) = self.rgba(base)?;
            let (r0, r1) = (r0.min(h), r1.min(h));
            if r1 <= r0 {
                return None;
            }
            return Some((w, r1 - r0, px[(r0 * w * 4) as usize..(r1 * w * 4) as usize].to_vec()));
        }
        decode_png(&std::fs::read(self.dir.join(&self.get(name).file)).ok()?)
    }

    /// A texture as PNG bytes, to go next to a level: the library's file, or a derived one made.
    pub fn png(&self, name: &str) -> Option<Vec<u8>> {
        if parse_derived(name).is_some() {
            let (w, h, px) = self.rgba(name)?;
            return Some(encode_png(w, h, &px));
        }
        std::fs::read(self.dir.join(&self.get(name).file)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_textures_are_rows_of_their_base() {
        let dir = std::env::temp_dir().join(format!("ow_tex_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // 2 x 4, each row its own grey
        let px: Vec<u8> = (0..4u8).flat_map(|r| [r * 60, r * 60, r * 60, 255, r * 60, r * 60, r * 60, 255]).collect();
        std::fs::write(dir.join("t.png"), encode_png(2, 4, &px)).unwrap();
        let lib = Library::load(&dir).unwrap();
        assert_eq!(lib.rgba("t").unwrap(), (2, 4, px.clone()));
        let name = derived_name("t", 1, 3, true);
        let info = lib.get(&name);
        assert_eq!((info.file.as_str(), info.wrap_v.as_str()), ("t-rows1-3.png", "mirror"));
        let (w, h, mid) = lib.rgba(&name).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(mid, px[8..24].to_vec());
        assert_eq!(decode_png(&lib.png(&name).unwrap()).unwrap(), (2, 2, mid));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
