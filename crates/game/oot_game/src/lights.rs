//! The point lights actors carry (`z_lights.c`): `LightContext`'s list and `sLightsBuffer`'s
//! nodes, and `Lights_BindPoint`, which turns a point light near a position into the directional
//! light the RSP draws with.
//!
//! In the C the `LightInfo` lives in the actor and the node points at it; here the context holds
//! each node's info and the actor writes it through its node (`LightContext::set_info`).
//! `Actor_Draw` binds every light of the list at the actor's position (`Lights_BindAll`), the
//! rooms without a position (only directional lights). The renderer's lights are the scene's
//! ambient and two directional ones, so what `bind_all` gives isn't drawn yet
//! (docs/adr/0023-navi-and-the-opening.md).

use glam::Vec3;

/// `LightType`.
pub const LIGHT_POINT_NOGLOW: u8 = 0;
pub const LIGHT_DIRECTIONAL: u8 = 1;
pub const LIGHT_POINT_GLOW: u8 = 2;
/// `LIGHTS_BUFFER_SIZE`.
pub const LIGHTS_BUFFER_SIZE: usize = 32;
/// `Lights_FindSlot`: a `Lights` holds 7 lights.
pub const MAX_BOUND_LIGHTS: usize = 7;

/// `LightInfo` with `LightPoint` params (`light.h`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LightInfo {
    pub ty: u8,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub color: [u8; 3],
    /// `drawGlow`, set by `Lights_GlowCheck` for a glowing light in view (not ported).
    pub draw_glow: u8,
    pub radius: i16,
}

impl LightInfo {
    /// `Lights_PointSetInfo`.
    pub fn point(ty: u8, x: i16, y: i16, z: i16, color: [u8; 3], radius: i16) -> LightInfo {
        LightInfo { ty, x, y, z, color, draw_glow: 0, radius }
    }
    /// `Lights_PointNoGlowSetInfo`.
    pub fn point_no_glow(x: i16, y: i16, z: i16, color: [u8; 3], radius: i16) -> LightInfo {
        Self::point(LIGHT_POINT_NOGLOW, x, y, z, color, radius)
    }
    /// `Lights_PointGlowSetInfo`.
    pub fn point_glow(x: i16, y: i16, z: i16, color: [u8; 3], radius: i16) -> LightInfo {
        Self::point(LIGHT_POINT_GLOW, x, y, z, color, radius)
    }
}

/// A `LightNode*`: an index into `sLightsBuffer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightNode(pub usize);

/// A light `Lights_BindPoint` bound: `l.dir` (not normalised: 120 over the distance) and `l.col`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundLight {
    pub dir: [i8; 3],
    pub color: [u8; 3],
}

/// `LightContext` and `sLightsBuffer`.
#[derive(Debug, Clone, PartialEq)]
pub struct LightContext {
    buf: [Option<LightInfo>; LIGHTS_BUFFER_SIZE],
    /// The list, `listHead` first.
    list: Vec<usize>,
    search_index: usize,
}

impl Default for LightContext {
    /// `LightContext_Init` (the ambient colour and fog it sets are the environment's here).
    fn default() -> LightContext {
        LightContext { buf: [None; LIGHTS_BUFFER_SIZE], list: Vec::new(), search_index: 0 }
    }
}

impl LightContext {
    /// `LightContext_InsertLight`: a free node (`Lights_FindBufSlot`) at the list's head.
    pub fn insert_light(&mut self, info: LightInfo) -> Option<LightNode> {
        if self.list.len() >= LIGHTS_BUFFER_SIZE {
            return None;
        }
        while self.buf[self.search_index].is_some() {
            self.search_index = (self.search_index + 1) % LIGHTS_BUFFER_SIZE;
        }
        let i = self.search_index;
        self.buf[i] = Some(info);
        self.list.insert(0, i);
        Some(LightNode(i))
    }

    /// `LightContext_RemoveLight`.
    pub fn remove_light(&mut self, node: Option<LightNode>) {
        if let Some(LightNode(i)) = node {
            self.list.retain(|&n| n != i);
            self.buf[i] = None;
        }
    }

    /// The actor's write of its `LightInfo` (`Lights_Point*SetInfo` on the struct the node
    /// points at).
    pub fn set_info(&mut self, node: Option<LightNode>, info: LightInfo) {
        if let Some(LightNode(i)) = node
            && let Some(slot) = self.buf.get_mut(i)
            && slot.is_some()
        {
            *slot = Some(info);
        }
    }

    pub fn info(&self, node: LightNode) -> Option<LightInfo> {
        self.buf.get(node.0).copied().flatten()
    }

    /// The lights in list order.
    pub fn lights(&self) -> impl Iterator<Item = LightInfo> + '_ {
        self.list.iter().filter_map(|&i| self.buf[i])
    }

    /// `Lights_BindAll`'s point lights at `pos` (`None`: none, as for the rooms), up to 7.
    pub fn bind_all(&self, pos: Option<Vec3>) -> Vec<BoundLight> {
        let mut out = Vec::new();
        for l in self.lights() {
            if l.ty != LIGHT_DIRECTIONAL
                && out.len() < MAX_BOUND_LIGHTS
                && let Some(b) = bind_point(&l, pos)
            {
                out.push(b);
            }
        }
        out
    }
}

/// `Lights_BindPoint`: within the radius, the colour scaled by `1 - (distance / radius)^2` and
/// the direction to the light, 120 long (or 120 times the offset within 1).
pub fn bind_point(l: &LightInfo, pos: Option<Vec3>) -> Option<BoundLight> {
    let v = pos?;
    let (x_diff, y_diff, z_diff) = (l.x as f32 - v.x, l.y as f32 - v.y, l.z as f32 - v.z);
    let mut scale = l.radius as f32;
    let mut pos_diff = x_diff * x_diff + y_diff * y_diff + z_diff * z_diff;
    if !(pos_diff < scale * scale) {
        return None;
    }
    pos_diff = pos_diff.sqrt();
    scale = pos_diff / scale;
    scale = 1.0 - scale * scale;
    let color = [(l.color[0] as f32 * scale) as u8, (l.color[1] as f32 * scale) as u8, (l.color[2] as f32 * scale) as u8];
    let s = if pos_diff < 1.0 { 120.0 } else { 120.0 / pos_diff };
    Some(BoundLight { dir: [(x_diff * s) as i32 as i8, (y_diff * s) as i32 as i8, (z_diff * s) as i32 as i8], color })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_light_binds_within_its_radius() {
        let mut ctx = LightContext::default();
        let a = ctx.insert_light(LightInfo::point_glow(0, 100, 0, [255, 255, 255], 100));
        let b = ctx.insert_light(LightInfo::point_no_glow(0, 0, 0, [255, 255, 255], 0));
        // The newest is the list's head.
        assert_eq!(ctx.lights().next().map(|l| l.ty), Some(LIGHT_POINT_NOGLOW));
        // 50 below it: half the radius, 3/4 of the colour, 120 up.
        let bound = ctx.bind_all(Some(Vec3::new(0.0, 50.0, 0.0)));
        assert_eq!(bound, vec![BoundLight { dir: [0, 120, 0], color: [191, 191, 191] }]);
        assert!(ctx.bind_all(Some(Vec3::new(0.0, -10.0, 0.0))).is_empty());
        assert!(ctx.bind_all(None).is_empty());
        ctx.remove_light(a);
        ctx.remove_light(b);
        assert_eq!(ctx.lights().count(), 0);
    }
}
