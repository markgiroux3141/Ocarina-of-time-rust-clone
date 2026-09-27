//! The synthetic test course, built as game-format collision (`CollisionHeader`, encoded to
//! bytes and decoded back so it takes the same path as a scene's collision).
//!
//! Layout (world units; Link faces -z at spawn, yaw 0x8000):
//!
//! ```text
//!  z=-1000 +-------------------------------------------------------------+
//!          | plateau (y=150)  |   stairs (+15 per 45, to y=120)  |  pit  |
//!          | ramp A 20°  ^    |                                  | y=-450|
//!          |                  |                                  |       |
//!          |        steep ramp B (30°, to y=120) ->     tall block (40)  |
//!          |                         spawn (0,0,0)                        |
//!          |             diagonal wall                                    |
//!  z=+1000 +-------------------------------------------------------------+
//!        x=-1000                                                     x=+1000
//! ```
//! Floors are split into 100-unit tiles, which is how the game's own meshes are usually cut
//! and gives the renderer a checkerboard.

use glam::Vec3;
use oot_core::collision::{CollisionBuilder, CollisionHeader};

/// Surface data word 0: floor type bits 13..17, wall type 21..25, floor property 26..29.
/// Word 1: sfx type 0..3, floor effect 4..5, light setting 6..10, echo 11..16.
pub const SURFACE_GROUND: (u32, u32) = (0, 0x0000_0000);
pub const SURFACE_STONE: (u32, u32) = (0, 0x0000_0002);

pub struct Course {
    pub collision: CollisionHeader,
    pub spawn: Vec3,
    pub spawn_yaw: i16,
    /// Named reference points used by the tests and the viewer.
    pub marks: Vec<(&'static str, Vec3)>,
}

struct B {
    b: CollisionBuilder,
    ground: u16,
    stone: u16,
}

impl B {
    /// Horizontal floor over [x0,x1]×[z0,z1] at height y, cut into `tile`-sized squares.
    fn floor(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y: f32, tile: f32) {
        let nx = ((x1 - x0) / tile).ceil() as i32;
        let nz = ((z1 - z0) / tile).ceil() as i32;
        for i in 0..nx {
            for k in 0..nz {
                let (a, b) = (x0 + i as f32 * tile, (x0 + (i + 1) as f32 * tile).min(x1));
                let (c, d) = (z0 + k as f32 * tile, (z0 + (k + 1) as f32 * tile).min(z1));
                // Counter-clockwise seen from above (+y).
                self.b.quad(Vec3::new(a, y, d), Vec3::new(b, y, d), Vec3::new(b, y, c), Vec3::new(a, y, c), self.ground);
            }
        }
    }

    /// Ramp over x∈[x0,x1] rising from y0 at z=z0 to y1 at z=z1, tiled along z.
    fn ramp_z(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y0: f32, y1: f32, tile: f32) {
        let n = ((z1 - z0).abs() / tile).ceil() as i32;
        for k in 0..n {
            let t0 = k as f32 / n as f32;
            let t1 = (k + 1) as f32 / n as f32;
            let (za, zb) = (z0 + (z1 - z0) * t0, z0 + (z1 - z0) * t1);
            let (ya, yb) = (y0 + (y1 - y0) * t0, y0 + (y1 - y0) * t1);
            let (p0, p1, p2, p3) = (Vec3::new(x0, ya, za), Vec3::new(x1, ya, za), Vec3::new(x1, yb, zb), Vec3::new(x0, yb, zb));
            // Orient so the normal points up.
            let n = (p1 - p0).cross(p2 - p0);
            if n.y > 0.0 {
                self.b.quad(p0, p1, p2, p3, self.ground);
            } else {
                self.b.quad(p0, p3, p2, p1, self.ground);
            }
        }
    }

    /// Ramp over z∈[z0,z1] rising from y0 at x=x0 to y1 at x=x1.
    fn ramp_x(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y0: f32, y1: f32, tile: f32) {
        let n = ((x1 - x0).abs() / tile).ceil() as i32;
        for k in 0..n {
            let t0 = k as f32 / n as f32;
            let t1 = (k + 1) as f32 / n as f32;
            let (xa, xb) = (x0 + (x1 - x0) * t0, x0 + (x1 - x0) * t1);
            let (ya, yb) = (y0 + (y1 - y0) * t0, y0 + (y1 - y0) * t1);
            let (p0, p1, p2, p3) = (Vec3::new(xa, ya, z0), Vec3::new(xb, yb, z0), Vec3::new(xb, yb, z1), Vec3::new(xa, ya, z1));
            let n = (p1 - p0).cross(p2 - p0);
            if n.y > 0.0 {
                self.b.quad(p0, p1, p2, p3, self.ground);
            } else {
                self.b.quad(p0, p3, p2, p1, self.ground);
            }
        }
    }

    /// Vertical wall from `a` to `b` (on the xz plane) between heights y0..y1, facing
    /// `outward` (a unit xz direction the normal should point to). Tiled every 100 along
    /// its length and height.
    fn wall(&mut self, a: (f32, f32), b: (f32, f32), y0: f32, y1: f32, outward: (f32, f32)) {
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let nl = (len / 100.0).ceil().max(1.0) as i32;
        let nh = ((y1 - y0) / 100.0).ceil().max(1.0) as i32;
        for i in 0..nl {
            for k in 0..nh {
                let t0 = i as f32 / nl as f32;
                let t1 = (i + 1) as f32 / nl as f32;
                let (ya, yb) = (y0 + (y1 - y0) * k as f32 / nh as f32, y0 + (y1 - y0) * (k + 1) as f32 / nh as f32);
                let pa = (a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0);
                let pb = (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1);
                let (p0, p1, p2, p3) = (Vec3::new(pa.0, ya, pa.1), Vec3::new(pb.0, ya, pb.1), Vec3::new(pb.0, yb, pb.1), Vec3::new(pa.0, yb, pa.1));
                let n = (p1 - p0).cross(p2 - p0);
                if n.x * outward.0 + n.z * outward.1 > 0.0 {
                    self.b.quad(p0, p1, p2, p3, self.stone);
                } else {
                    self.b.quad(p0, p3, p2, p1, self.stone);
                }
            }
        }
    }

    /// Axis-aligned box (top at y1, walls down to y0) over [x0,x1]×[z0,z1].
    fn block(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y0: f32, y1: f32) {
        self.floor(x0, x1, z0, z1, y1, 100.0);
        self.wall((x0, z0), (x1, z0), y0, y1, (0.0, -1.0));
        self.wall((x0, z1), (x1, z1), y0, y1, (0.0, 1.0));
        self.wall((x0, z0), (x0, z1), y0, y1, (-1.0, 0.0));
        self.wall((x1, z0), (x1, z1), y0, y1, (1.0, 0.0));
    }
}

pub fn build() -> Course {
    let mut b = CollisionBuilder::new();
    let ground = b.surface(SURFACE_GROUND.0, SURFACE_GROUND.1);
    let stone = b.surface(SURFACE_STONE.0, SURFACE_STONE.1);
    let mut c = B { b, ground, stone };

    // Ground, with a pit cut out at x∈[400,800], z∈[-800,-400].
    c.floor(-1000.0, 1000.0, -400.0, 1000.0, 0.0, 100.0);
    c.floor(-1000.0, 400.0, -1000.0, -400.0, 0.0, 100.0);
    c.floor(800.0, 1000.0, -1000.0, -400.0, 0.0, 100.0);
    c.floor(400.0, 800.0, -1000.0, -800.0, 0.0, 100.0);
    // Pit: floor at -450, walls facing inwards.
    c.floor(400.0, 800.0, -800.0, -400.0, -450.0, 100.0);
    c.wall((400.0, -800.0), (800.0, -800.0), -450.0, 0.0, (0.0, 1.0));
    c.wall((400.0, -400.0), (800.0, -400.0), -450.0, 0.0, (0.0, -1.0));
    c.wall((400.0, -800.0), (400.0, -400.0), -450.0, 0.0, (1.0, 0.0));
    c.wall((800.0, -800.0), (800.0, -400.0), -450.0, 0.0, (-1.0, 0.0));

    // Plateau (y=150) with ramp A (≈20.6°) up its +z side.
    c.block(-900.0, -500.0, -900.0, -500.0, 0.0, 150.0);
    c.ramp_z(-800.0, -600.0, -100.0, -500.0, 0.0, 150.0, 100.0);
    // Ramp side walls down to the ground.
    for x in [-800.0, -600.0] {
        let out = if x < -700.0 { (-1.0, 0.0) } else { (1.0, 0.0) };
        // Triangular side: approximate with one quad per ramp tile.
        for k in 0..4 {
            let (za, zb) = (-100.0 - 100.0 * k as f32, -200.0 - 100.0 * k as f32);
            let (ya, yb) = (150.0 * k as f32 / 4.0, 150.0 * (k + 1) as f32 / 4.0);
            let (p0, p1, p2) = (Vec3::new(x, 0.0, za), Vec3::new(x, 0.0, zb), Vec3::new(x, yb, zb));
            let p3 = Vec3::new(x, ya, za);
            let n = (p1 - p0).cross(p2 - p0);
            if n.x * out.0 > 0.0 {
                c.b.quad(p0, p1, p2, p3, stone);
            } else {
                c.b.quad(p0, p3, p2, p1, stone);
            }
        }
    }

    // Stairs: eight 15-unit steps, 45 deep, rising along -z from z=-450 (15 is under the
    // step-up limit; see the findings).
    let mut y = 0.0;
    for i in 0..8 {
        let z1 = -450.0 - 45.0 * i as f32;
        let z0 = z1 - 45.0;
        let top = y + 15.0;
        c.floor(0.0, 300.0, z0, z1, top, 100.0);
        c.wall((0.0, z1), (300.0, z1), y, top, (0.0, 1.0));
        c.wall((0.0, z0), (0.0, z1), 0.0, top, (-1.0, 0.0));
        c.wall((300.0, z0), (300.0, z1), 0.0, top, (1.0, 0.0));
        y = top;
    }
    // Back of the stairs drops to the ground.
    c.wall((0.0, -810.0), (300.0, -810.0), 0.0, y, (0.0, -1.0));

    // Steep ramp B (30°) rising along +x to y=120 at x=-100, then a drop.
    let run = 120.0 / (30.0f32).to_radians().tan();
    c.ramp_x(-100.0 - run, -100.0, -250.0, -100.0, 0.0, 120.0, 50.0);
    c.wall((-100.0, -250.0), (-100.0, -100.0), 0.0, 120.0, (1.0, 0.0));

    // Tall block: 40 high, too tall to step onto.
    c.block(400.0, 600.0, 150.0, 350.0, 0.0, 40.0);

    // Diagonal wall at 30° to the x axis: a slab 20 thick with end caps.
    let (dx, dz) = ((30.0f32).to_radians().cos(), (30.0f32).to_radians().sin());
    let (nx, nz) = (-dz, dx); // +z-ish face normal
    let (ax, az) = (-500.0, 300.0);
    let (bx, bz) = (ax + 400.0 * dx, az + 400.0 * dz);
    let off = |x: f32, z: f32, s: f32| (x + nx * s, z + nz * s);
    let (a1, b1, a0, b0) = (off(ax, az, 10.0), off(bx, bz, 10.0), off(ax, az, -10.0), off(bx, bz, -10.0));
    c.wall(a1, b1, 0.0, 120.0, (nx, nz));
    c.wall(a0, b0, 0.0, 120.0, (-nx, -nz));
    c.wall(a0, a1, 0.0, 120.0, (-dx, -dz));
    c.wall(b0, b1, 0.0, 120.0, (dx, dz));
    // Top of the slab.
    let top = c.ground;
    c.b.quad(Vec3::new(a1.0, 120.0, a1.1), Vec3::new(b1.0, 120.0, b1.1), Vec3::new(b0.0, 120.0, b0.1), Vec3::new(a0.0, 120.0, a0.1), top);

    // Boundary walls facing inwards.
    c.wall((-1000.0, -1000.0), (1000.0, -1000.0), 0.0, 300.0, (0.0, 1.0));
    c.wall((-1000.0, 1000.0), (1000.0, 1000.0), 0.0, 300.0, (0.0, -1.0));
    c.wall((-1000.0, -1000.0), (-1000.0, 1000.0), 0.0, 300.0, (1.0, 0.0));
    c.wall((1000.0, -1000.0), (1000.0, 1000.0), 0.0, 300.0, (-1.0, 0.0));

    let header = c.b.finish();
    // Round-trip through the binary format so the game code reads exactly what a scene
    // file would contain.
    let bytes = header.encode(2);
    let collision = CollisionHeader::parse(&bytes, 2, 0).expect("course collision round-trips");
    Course {
        collision,
        spawn: Vec3::ZERO,
        spawn_yaw: -0x8000,
        marks: vec![
            ("spawn", Vec3::ZERO),
            ("plateau", Vec3::new(-700.0, 150.0, -700.0)),
            ("ramp A foot", Vec3::new(-700.0, 0.0, -50.0)),
            ("stairs foot", Vec3::new(150.0, 0.0, -400.0)),
            ("ramp B foot", Vec3::new(-100.0 - run - 50.0, 0.0, -175.0)),
            ("pit", Vec3::new(600.0, -450.0, -600.0)),
            ("tall block", Vec3::new(500.0, 40.0, 250.0)),
        ],
    }
}
