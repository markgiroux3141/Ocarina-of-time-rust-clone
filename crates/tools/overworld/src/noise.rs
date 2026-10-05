//! Smooth value noise: deterministic, about -1..1, varying over one unit. Used to break up
//! texture repetition (wall u speed, shade variation) and, later, for terrain brushes.

fn hash(i: i64, j: i64, k: i64, seed: u32) -> f64 {
    let mut h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (k as u64).wrapping_mul(0x1656_67B1_9E37_79F9)
        ^ (seed as u64).wrapping_mul(0x27D4_EB2F_1656_67C5);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 32;
    (h >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// Trilinear value noise with smoothstep fades: continuous, -1..1, features about 1 unit apart.
pub fn value3(p: [f64; 3], seed: u32) -> f64 {
    let f = p.map(f64::floor);
    let t = [p[0] - f[0], p[1] - f[1], p[2] - f[2]].map(|x| x * x * (3.0 - 2.0 * x));
    let (i, j, k) = (f[0] as i64, f[1] as i64, f[2] as i64);
    let mut out = 0.0;
    for (dx, wx) in [(0, 1.0 - t[0]), (1, t[0])] {
        for (dy, wy) in [(0, 1.0 - t[1]), (1, t[1])] {
            for (dz, wz) in [(0, 1.0 - t[2]), (1, t[2])] {
                out += wx * wy * wz * hash(i + dx, j + dy, k + dz, seed);
            }
        }
    }
    out
}

/// Two octaves of value noise, about -1..1.
pub fn fbm3(p: [f64; 3], seed: u32) -> f64 {
    let q = [p[0] * 2.03 + 17.1, p[1] * 2.03 - 9.7, p[2] * 2.03 + 3.3];
    (value3(p, seed) + 0.5 * value3(q, seed.wrapping_add(1))) / 1.5
}

/// Three octaves of value noise, -1..1, stretched to use that range (averaged octaves of value
/// noise stay near 0: 99% of raw values are within 0.54), softly limited so hills round off
/// instead of clipping. Rolling ground with smaller bumps on it.
pub fn relief(p: [f64; 2], seed: u32) -> f64 {
    let mut out = 0.0;
    let (mut f, mut a) = (1.0, 1.0);
    for o in 0..3u32 {
        out += a * value3([p[0] * f + 31.7 * o as f64, p[1] * f - 12.9 * o as f64, 0.61], seed.wrapping_add(o * 101));
        f *= 2.1;
        a *= 0.45;
    }
    (2.6 * out / (1.0 + 0.45 + 0.45 * 0.45)).tanh()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_bounded_continuous_and_varied() {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for i in 0..2000 {
            let p = [i as f64 * 0.137, i as f64 * 0.071, 0.5];
            let v = fbm3(p, 7);
            lo = lo.min(v);
            hi = hi.max(v);
            let w = fbm3([p[0] + 1e-4, p[1], p[2]], 7);
            assert!((v - w).abs() < 1e-2, "continuous");
        }
        assert!(lo >= -1.0 && hi <= 1.0 && hi - lo > 0.8, "{lo} {hi}");
    }

    #[test]
    fn relief_uses_its_range() {
        let mut v: Vec<f64> = (0..200_000).map(|i| relief([(i % 500) as f64 * 0.0731, (i / 500) as f64 * 0.0697], 5)).collect();
        assert!(v.iter().all(|x| x.abs() < 1.0));
        v.sort_by(|a, b| a.abs().total_cmp(&b.abs()));
        let n = v.len();
        let (p50, p99) = (v[n / 2].abs(), v[n * 99 / 100].abs());
        assert!(p50 > 0.3 && p99 > 0.8, "p50 {p50:.2} p99 {p99:.2}");
        // both ways
        assert!(v.iter().filter(|&&x| x > 0.5).count() > n / 10 && v.iter().filter(|&&x| x < -0.5).count() > n / 10);
    }
}

