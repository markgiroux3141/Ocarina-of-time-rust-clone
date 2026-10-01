//! WAV files (16-bit PCM), for tools and tests.

/// A 16-bit PCM WAV of `channels` interleaved channels at `rate` Hz.
pub fn wav_bytes(samples: &[i16], channels: u16, rate: u32) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut w = Vec::with_capacity(44 + data_len);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 2 * channels as u32).to_le_bytes());
    w.extend_from_slice(&(2 * channels).to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}
