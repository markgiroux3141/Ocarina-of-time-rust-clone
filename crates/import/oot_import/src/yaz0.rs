//! Yaz0 decompression (used by retail ROMs; the debug ROM ships uncompressed).

use anyhow::{Result, bail};

pub fn is_yaz0(data: &[u8]) -> bool {
    data.len() >= 16 && &data[0..4] == b"Yaz0"
}

pub fn decompress(src: &[u8]) -> Result<Vec<u8>> {
    if !is_yaz0(src) {
        bail!("missing Yaz0 magic");
    }
    let out_len = u32::from_be_bytes([src[4], src[5], src[6], src[7]]) as usize;
    let mut out = Vec::with_capacity(out_len);
    let mut pos = 16usize;
    let mut code = 0u8;
    let mut bits_left = 0;

    while out.len() < out_len {
        if bits_left == 0 {
            code = *src.get(pos).ok_or_else(|| anyhow::anyhow!("yaz0: truncated"))?;
            pos += 1;
            bits_left = 8;
        }
        if code & 0x80 != 0 {
            out.push(*src.get(pos).ok_or_else(|| anyhow::anyhow!("yaz0: truncated"))?);
            pos += 1;
        } else {
            let b1 = *src.get(pos).ok_or_else(|| anyhow::anyhow!("yaz0: truncated"))? as usize;
            let b2 = *src.get(pos + 1).ok_or_else(|| anyhow::anyhow!("yaz0: truncated"))? as usize;
            pos += 2;
            let dist = ((b1 & 0xF) << 8 | b2) + 1;
            let len = if b1 >> 4 == 0 {
                let b3 = *src.get(pos).ok_or_else(|| anyhow::anyhow!("yaz0: truncated"))? as usize;
                pos += 1;
                b3 + 0x12
            } else {
                (b1 >> 4) + 2
            };
            if dist > out.len() {
                bail!("yaz0: back-reference before start of output");
            }
            let start = out.len() - dist;
            // Byte-by-byte on purpose: runs may overlap their own output.
            for i in 0..len {
                let b = out[start + i];
                out.push(b);
            }
        }
        code <<= 1;
        bits_left -= 1;
    }
    out.truncate(out_len);
    Ok(out)
}
