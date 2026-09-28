//! What a frame is drawn with: the camera, the lights and the fog.

use glam::{Mat4, Vec3};

pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,
    /// Near and far planes; `None` scales them with the orbit distance (viewer default).
    pub clip: Option<(f32, f32)>,
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let dir = Vec3::new(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos());
        self.target + dir * self.distance
    }
    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.eye(), self.target, Vec3::Y)
    }
    pub fn clip_planes(&self) -> (f32, f32) {
        self.clip.unwrap_or((self.distance * 0.02, self.distance * 20.0))
    }
    pub fn proj(&self, aspect: f32) -> Mat4 {
        let (n, f) = self.clip_planes();
        glam::camera::rh::proj::directx::perspective(self.fov_y, aspect, n, f)
    }
}

/// N64 vertex fog (F3DEX2): each vertex's fog factor is `z_ndc * fm + fo` (in 1/256ths,
/// clamped), with `z_ndc` the OpenGL-style depth of the game's projection (`near`..`far`).
/// The blender then mixes the fog colour in by that factor (`G_RM_FOG_SHADE_A`).
#[derive(Debug, Clone, Copy)]
pub struct Fog {
    pub color: Vec3,
    /// `gSPFogFactor` multiplier and offset.
    pub multiplier: f32,
    pub offset: f32,
    /// The projection the factor is computed against (the game's `zNear` and `fogFar`).
    pub near: f32,
    pub far: f32,
}

/// Directional lights and ambient in world space, as `Lights_Draw` loads them for F3DEX2:
/// shade = ambient + sum of colour * max(0, n . dir), `dir` pointing towards the light.
pub struct Lighting {
    pub dir: Vec3,
    pub color: Vec3,
    pub dir2: Vec3,
    pub color2: Vec3,
    pub ambient: Vec3,
    pub fog: Option<Fog>,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            dir: Vec3::new(0.45, 0.8, 0.6).normalize(),
            color: Vec3::splat(0.7),
            dir2: Vec3::Y,
            color2: Vec3::ZERO,
            ambient: Vec3::splat(0.38),
            fog: None,
        }
    }
}
