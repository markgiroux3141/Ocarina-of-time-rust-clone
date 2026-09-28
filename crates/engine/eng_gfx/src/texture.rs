//! N64 texture formats and decoded images: the parts of a texture the renderer and the asset
//! pack need. Decoding from TMEM lives in `eng_gbi::texture`.

pub const G_IM_FMT_RGBA: u8 = 0;
pub const G_IM_FMT_YUV: u8 = 1;
pub const G_IM_FMT_CI: u8 = 2;
pub const G_IM_FMT_IA: u8 = 3;
pub const G_IM_FMT_I: u8 = 4;

pub const G_IM_SIZ_4B: u8 = 0;
pub const G_IM_SIZ_8B: u8 = 1;
pub const G_IM_SIZ_16B: u8 = 2;
pub const G_IM_SIZ_32B: u8 = 3;

pub fn format_name(fmt: u8, siz: u8) -> String {
    let f = match fmt {
        G_IM_FMT_RGBA => "rgba",
        G_IM_FMT_YUV => "yuv",
        G_IM_FMT_CI => "ci",
        G_IM_FMT_IA => "ia",
        G_IM_FMT_I => "i",
        _ => "?",
    };
    let s = match siz {
        0 => "4",
        1 => "8",
        2 => "16",
        _ => "32",
    };
    format!("{f}{s}")
}

/// Bits per texel for a G_IM_SIZ value.
pub fn bits_per_texel(siz: u8) -> usize {
    4 << siz
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum WrapMode {
    #[default]
    Repeat,
    Mirror,
    Clamp,
}

impl WrapMode {
    pub fn from_cm(cm: u8, mask: u8) -> WrapMode {
        // G_TX_MIRROR = 1, G_TX_CLAMP = 2. With mask 0 the coordinate can't wrap, so it clamps.
        if cm & 2 != 0 || mask == 0 {
            WrapMode::Clamp
        } else if cm & 1 != 0 {
            WrapMode::Mirror
        } else {
            WrapMode::Repeat
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
