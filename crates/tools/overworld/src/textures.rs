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
    /// Drawn on a surface it lies on (N64 decal depth mode), such as a door's shadow.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub decal: bool,
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
        TexInfo { file: format!("{name}.png"), size: vec![], wrap_u: repeat(), wrap_v: repeat(), alpha: opaque(), opacity: 1.0, cull: back(), decal: false }
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

/// A floor texture with a decal drawn over it, for blending in by vertex weight (the ground under a
/// dirt path): `base` with `over` on top, `repeats` times across each way, its columns `cols`
/// (the strip's middle) mirrored across so it tiles, multiplied by `tint`, at `opacity` times its
/// own alpha. At least 128 texels across (four times Kokiri's 32-texel camo), so the overlay keeps
/// its detail.
pub fn composite_name(base: &str, over: &str, tint: [f64; 3], opacity: f64, repeats: u32, cols: [u32; 2]) -> String {
    let t = tint.map(|x| x.round().clamp(0.0, 255.0) as u8);
    format!("{base}+{over}@{:02x}{:02x}{:02x}-{}-{repeats}-{}-{}", t[0], t[1], t[2], (opacity * 100.0).round() as u32, cols[0], cols[1])
}

struct Composite<'a> {
    base: &'a str,
    over: &'a str,
    tint: [u8; 3],
    opacity: f64,
    repeats: u32,
    cols: [u32; 2],
}

fn parse_composite(name: &str) -> Option<Composite<'_>> {
    let (pair, rest) = name.rsplit_once('@')?;
    let (base, over) = pair.split_once('+')?;
    let mut it = rest.split('-');
    let hex = it.next()?;
    if hex.len() != 6 {
        return None;
    }
    let c = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let tint = [c(0)?, c(2)?, c(4)?];
    let opacity = it.next()?.parse::<f64>().ok()? / 100.0;
    let repeats = it.next()?.parse().ok()?;
    let cols = [it.next()?.parse().ok()?, it.next()?.parse().ok()?];
    Some(Composite { base, over, tint, opacity, repeats, cols })
}

/// Bilinear sample of an RGBA8 image at texel coordinates (repeating).
fn bilinear(w: u32, h: u32, px: &[u8], x: f64, y: f64) -> [f64; 4] {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |i: i64, j: i64| {
        let (i, j) = (i.rem_euclid(w as i64) as usize, j.rem_euclid(h as i64) as usize);
        let k = (j * w as usize + i) * 4;
        [px[k] as f64, px[k + 1] as f64, px[k + 2] as f64, px[k + 3] as f64]
    };
    let (a, b, c, d) = (at(x0 as i64, y0 as i64), at(x0 as i64 + 1, y0 as i64), at(x0 as i64, y0 as i64 + 1), at(x0 as i64 + 1, y0 as i64 + 1));
    [0, 1, 2, 3].map(|k| (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy)
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

/// Textures by name: one folder's (`load`), or several themes' folders merged (`load_all`), each
/// texture read from the folder it came from.
pub struct Library {
    /// The first folder: where textures not listed in any `textures.json` are looked for.
    pub dir: PathBuf,
    pub info: BTreeMap<String, TexInfo>,
    /// The folder each listed texture came from, when there are several.
    from: BTreeMap<String, PathBuf>,
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
        Ok(Library { dir: dir.to_path_buf(), info, from: BTreeMap::new() })
    }

    /// Several folders' libraries as one (the themes' `kf_`, `kak_`...). A name in more than one
    /// is the first folder's.
    pub fn load_all(dirs: &[PathBuf]) -> Result<Library, String> {
        let first = dirs.first().ok_or("no texture folders")?;
        let mut lib = Library::load(first)?;
        for d in &dirs[1..] {
            let other = Library::load(d)?;
            for (name, info) in other.info {
                if !lib.info.contains_key(&name) {
                    lib.from.insert(name.clone(), d.clone());
                    lib.info.insert(name, info);
                }
            }
        }
        Ok(lib)
    }

    /// Where a library texture's file is (not for derived or composite names, which are made).
    pub fn path(&self, name: &str) -> PathBuf {
        self.from.get(name).unwrap_or(&self.dir).join(&self.get(name).file)
    }

    pub fn get(&self, name: &str) -> TexInfo {
        if let Some(c) = parse_composite(name) {
            let b = self.get(c.base);
            let o = self.get(c.over);
            let stem = |f: &str| f.strip_suffix(".png").unwrap_or(f).to_string();
            let tag = name.rsplit_once('@').map_or("", |x| x.1);
            return TexInfo {
                file: format!("{}+{}-{tag}.png", stem(&b.file), stem(&o.file)),
                size: b.size.iter().map(|x| x * 128u32.div_ceil(*x.max(&1)).max(1)).collect(),
                decal: false,
                ..b
            };
        }
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
        if let Some(c) = parse_composite(name) {
            let (bw, bh, bpx) = self.rgba(c.base)?;
            let (ow, oh, opx) = self.rgba(c.over)?;
            let k = 128u32.div_ceil(bw).max(1);
            let (w, h) = (bw * k, bh * k);
            let (c0, c1) = (c.cols[0].min(ow - 1) as f64, c.cols[1].min(ow) as f64);
            let period = (w / c.repeats.max(1)) as f64;
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let g = bilinear(bw, bh, &bpx, (x as f64 + 0.5) / k as f64 - 0.5, (y as f64 + 0.5) / k as f64 - 0.5);
                    // across: the strip's middle columns, there and back; along: its rows, repeating
                    let u = ((x as f64 + 0.5) % period) / period * 2.0;
                    let u = if u < 1.0 { u } else { 2.0 - u };
                    let ox = c0 + u * (c1 - 1.0 - c0);
                    let oy = ((y as f64 + 0.5) % period) / period * oh as f64 - 0.5;
                    let o = bilinear(ow, oh, &opx, ox, oy);
                    let a = (o[3] / 255.0 * c.opacity).clamp(0.0, 1.0);
                    for k in 0..3 {
                        let over = o[k] * c.tint[k] as f64 / 255.0;
                        out.push((g[k] * (1.0 - a) + over * a).round().clamp(0.0, 255.0) as u8);
                    }
                    out.push(255);
                }
            }
            return Some((w, h, out));
        }
        if let Some((base, r0, r1, _)) = parse_derived(name) {
            let (w, h, px) = self.rgba(base)?;
            let (r0, r1) = (r0.min(h), r1.min(h));
            if r1 <= r0 {
                return None;
            }
            return Some((w, r1 - r0, px[(r0 * w * 4) as usize..(r1 * w * 4) as usize].to_vec()));
        }
        decode_png(&std::fs::read(self.path(name)).ok()?)
    }

    /// A texture as PNG bytes, to go next to a level: the library's file, or a derived one made.
    pub fn png(&self, name: &str) -> Option<Vec<u8>> {
        if parse_composite(name).is_some() || parse_derived(name).is_some() {
            let (w, h, px) = self.rgba(name)?;
            return Some(encode_png(w, h, &px));
        }
        std::fs::read(self.path(name)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composites_are_the_base_with_the_overlay_drawn_over_it() {
        let dir = std::env::temp_dir().join(format!("ow_tex_comp_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // a 2 x 2 grey base, and a 4 x 2 white overlay opaque in its middle two columns only
        std::fs::write(dir.join("g.png"), encode_png(2, 2, &[100u8, 100, 100, 255].repeat(4))).unwrap();
        let over: Vec<u8> = (0..2).flat_map(|_| (0..4).flat_map(|c| [255, 255, 255, if c == 1 || c == 2 { 255 } else { 0 }])).collect();
        std::fs::write(dir.join("o.png"), encode_png(4, 2, &over)).unwrap();
        let lib = Library::load(&dir).unwrap();
        let name = composite_name("g", "o", [200.0, 100.0, 0.0], 0.5, 1, [1, 3]);
        assert_eq!(name, "g+o@c86400-50-1-1-3");
        let (w, h, px) = lib.rgba(&name).unwrap();
        assert_eq!((w, h), (128, 128), "at least 128 across");
        // the middle columns, tinted, half over the grey: (100 + 200) / 2, (100 + 100) / 2, 100 / 2
        assert_eq!(&px[0..4], &[150, 100, 50, 255]);
        assert!(px.chunks(4).all(|c| c == [150, 100, 50, 255]), "the opaque middle covers the tile");
        assert_eq!(lib.get(&name).file, "g+o-c86400-50-1-1-3.png");
        assert!(lib.png(&name).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

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
