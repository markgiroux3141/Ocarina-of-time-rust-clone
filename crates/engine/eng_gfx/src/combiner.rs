//! RDP colour combiner: `(A - B) * C + D` for colour and alpha, over one or two cycles.
//!
//! Selectors are normalised into a single `Input` numbering so a shader can evaluate
//! any combiner with one switch statement.

/// Unified combiner inputs. The numeric values are shared with the viewer's WGSL shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Input {
    Combined = 0,
    Texel0 = 1,
    Texel1 = 2,
    Prim = 3,
    Shade = 4,
    Env = 5,
    One = 6,
    Zero = 7,
    Noise = 8,
    KeyCenter = 9,
    K4 = 10,
    KeyScale = 11,
    CombinedAlpha = 12,
    Texel0Alpha = 13,
    Texel1Alpha = 14,
    PrimAlpha = 15,
    ShadeAlpha = 16,
    EnvAlpha = 17,
    LodFraction = 18,
    PrimLodFrac = 19,
    K5 = 20,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Cycle {
    pub a: Input,
    pub b: Input,
    pub c: Input,
    pub d: Input,
    pub aa: Input,
    pub ab: Input,
    pub ac: Input,
    pub ad: Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Combiner {
    pub raw: u64,
    pub cycles: [Cycle; 2],
}

use Input::*;

fn color_a(v: u32) -> Input {
    [Combined, Texel0, Texel1, Prim, Shade, Env, One, Noise].get(v as usize).copied().unwrap_or(Zero)
}
fn color_b(v: u32) -> Input {
    [Combined, Texel0, Texel1, Prim, Shade, Env, KeyCenter, K4].get(v as usize).copied().unwrap_or(Zero)
}
fn color_c(v: u32) -> Input {
    [
        Combined, Texel0, Texel1, Prim, Shade, Env, KeyScale, CombinedAlpha, Texel0Alpha, Texel1Alpha, PrimAlpha,
        ShadeAlpha, EnvAlpha, LodFraction, PrimLodFrac, K5,
    ]
    .get(v as usize)
    .copied()
    .unwrap_or(Zero)
}
fn color_d(v: u32) -> Input {
    [Combined, Texel0, Texel1, Prim, Shade, Env, One, Zero][v as usize & 7]
}
fn alpha_abd(v: u32) -> Input {
    [Combined, Texel0, Texel1, Prim, Shade, Env, One, Zero][v as usize & 7]
}
fn alpha_c(v: u32) -> Input {
    [LodFraction, Texel0, Texel1, Prim, Shade, Env, PrimLodFrac, Zero][v as usize & 7]
}

impl Combiner {
    /// `raw` is the 56 bits of a G_SETCOMBINE command: w0's low 24 bits, then w1.
    pub fn decode(raw: u64) -> Combiner {
        let w0 = (raw >> 32) as u32;
        let w1 = raw as u32;
        let c0 = Cycle {
            a: color_a((w0 >> 20) & 0xF),
            c: color_c((w0 >> 15) & 0x1F),
            aa: alpha_abd((w0 >> 12) & 7),
            ac: alpha_c((w0 >> 9) & 7),
            b: color_b((w1 >> 28) & 0xF),
            d: color_d((w1 >> 15) & 7),
            ab: alpha_abd((w1 >> 12) & 7),
            ad: alpha_abd((w1 >> 9) & 7),
        };
        let c1 = Cycle {
            a: color_a((w0 >> 5) & 0xF),
            c: color_c(w0 & 0x1F),
            aa: alpha_abd((w1 >> 21) & 7),
            ac: alpha_c((w1 >> 18) & 7),
            b: color_b((w1 >> 24) & 0xF),
            d: color_d((w1 >> 6) & 7),
            ab: alpha_abd((w1 >> 3) & 7),
            ad: alpha_abd(w1 & 7),
        };
        Combiner { raw, cycles: [c0, c1] }
    }

    pub fn uses_texel(&self, which: usize, two_cycle: bool) -> bool {
        let (t, ta) = if which == 0 { (Texel0, Texel0Alpha) } else { (Texel1, Texel1Alpha) };
        let n = if two_cycle { 2 } else { 1 };
        self.cycles[..n].iter().any(|c| {
            [c.a, c.b, c.c, c.d, c.aa, c.ab, c.ac, c.ad].iter().any(|&i| i == t || i == ta)
        })
    }

    /// Packs the 16 selectors (cycle 0 colour a..d, alpha a..d, then cycle 1) for a shader.
    pub fn selectors(&self) -> [u32; 16] {
        let mut out = [0u32; 16];
        for (ci, c) in self.cycles.iter().enumerate() {
            let s = [c.a, c.b, c.c, c.d, c.aa, c.ab, c.ac, c.ad];
            for (i, v) in s.iter().enumerate() {
                out[ci * 8 + i] = *v as u32;
            }
        }
        out
    }

    pub fn describe(&self, two_cycle: bool) -> String {
        let n = if two_cycle { 2 } else { 1 };
        self.cycles[..n]
            .iter()
            .map(|c| {
                format!(
                    "({:?} - {:?}) * {:?} + {:?} | ({:?} - {:?}) * {:?} + {:?}",
                    c.a, c.b, c.c, c.d, c.aa, c.ab, c.ac, c.ad
                )
            })
            .collect::<Vec<_>>()
            .join("  ;  ")
    }
}

/// Encode one G_SETCOMBINE from GBI-style selector numbers (as in `gsDPSetCombineLERP`).
#[allow(clippy::too_many_arguments)]
pub fn encode(c0: [u32; 8], c1: [u32; 8]) -> u64 {
    let [a0, b0, c0c, d0, aa0, ab0, ac0, ad0] = c0;
    let [a1, b1, c1c, d1, aa1, ab1, ac1, ad1] = c1;
    let w0 = ((a0 & 0xF) << 20)
        | ((c0c & 0x1F) << 15)
        | ((aa0 & 7) << 12)
        | ((ac0 & 7) << 9)
        | ((a1 & 0xF) << 5)
        | (c1c & 0x1F);
    let w1 = ((b0 & 0xF) << 28)
        | ((b1 & 0xF) << 24)
        | ((aa1 & 7) << 21)
        | ((ac1 & 7) << 18)
        | ((d0 & 7) << 15)
        | ((ab0 & 7) << 12)
        | ((ad0 & 7) << 9)
        | ((d1 & 7) << 6)
        | ((ab1 & 7) << 3)
        | (ad1 & 7);
    ((w0 as u64) << 32) | w1 as u64
}

/// G_CC_MODULATEIDECALA, G_CC_MODULATEIA_PRIM2 (the combiner set by setup DL 25).
pub fn cc_modulate_idecala_modulateia_prim2() -> u64 {
    // Raw GBI selector numbers: colour A/B/C/D: TEXEL0=1, SHADE=4, PRIMITIVE=3, COMBINED=0,
    // ZERO = 15 (A/B), 31 (C), 7 (D). Alpha A/B/D ZERO = 7, alpha C: PRIMITIVE = 3.
    encode([1, 15, 4, 7, 7, 7, 7, 1], [0, 15, 3, 7, 0, 7, 3, 7])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_setup_dl25() {
        let c = Combiner::decode(cc_modulate_idecala_modulateia_prim2());
        assert_eq!(c.cycles[0].a, Texel0);
        assert_eq!(c.cycles[0].c, Shade);
        assert_eq!(c.cycles[0].d, Zero);
        assert_eq!(c.cycles[0].ad, Texel0);
        assert_eq!(c.cycles[1].a, Combined);
        assert_eq!(c.cycles[1].c, Prim);
        assert_eq!(c.cycles[1].ac, Prim);
        assert!(c.uses_texel(0, true));
        assert!(!c.uses_texel(1, true));
    }
}
