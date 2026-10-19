//! Flattening a frame's draws into one mesh: each draw's mesh posed by its transform and bones
//! (as the renderer poses it), with what the draw sets for itself baked in (its dynamic segments'
//! colours and tile scrolls, its vertex colours, its own texels). What's left is a still mesh that
//! draws as that frame did: the game editor's captures of its actors (docs/EDITOR-ACTORS.md).
//! A draw's point lights and fog are the frame's, not the mesh's: they're left out.

use std::hash::{Hash, Hasher};

use glam::{Mat3, Mat4, Vec3};

use crate::draw::{Batch, DrawList, NO_BONE, TextureImage, Vertex};
use crate::submit::{DrawCmd, DrawImage};
use crate::texture::DecodedImage;

fn image_hash(img: &DrawImage) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (img.width, img.height, &img.rgba[..]).hash(&mut h);
    h.finish()
}

fn texture_of(img: &DrawImage, like: Option<&TextureImage>) -> TextureImage {
    TextureImage {
        image: DecodedImage { width: img.width, height: img.height, rgba: img.rgba.to_vec() },
        fmt: like.map_or(0, |t| t.fmt),
        siz: like.map_or(3, |t| t.siz),
        hash: image_hash(img),
        source_segments: 0,
        source_addr: None,
    }
}

/// The draws (each with its resolved mesh), in drawing order, as one mesh whose vertices are in
/// the space `root` maps the world to. Its vertices have no bones.
pub fn flatten<'a>(draws: impl IntoIterator<Item = (&'a DrawCmd, &'a DrawList)>, root: Mat4) -> DrawList {
    let mut out = DrawList::default();
    for (cmd, mesh) in draws {
        let p = &cmd.params;
        // its textures, the ones the draw replaces by source swapped for its own texels
        let tex: Vec<usize> = mesh
            .textures
            .iter()
            .map(|t| match p.texture_images.iter().find(|s| t.source_addr == Some(s.source)) {
                Some(s) => out.intern_texture(image_hash(&s.image), || texture_of(&s.image, Some(t))),
                None => out.intern_texture(t.hash, || t.clone()),
            })
            .collect();
        let slot0 = p.image.as_ref().map(|img| out.intern_texture(image_hash(img), || texture_of(img, None)));
        // its materials with the draw's segment values written in
        let mut uv_off = Vec::with_capacity(mesh.materials.len());
        let mats: Vec<usize> = mesh
            .materials
            .iter()
            .map(|m| {
                let mut m = m.clone();
                let off = match &p.segments {
                    Some(v) => {
                        (m.env, m.prim) = m.colors(v);
                        m.uv_offsets(v)
                    }
                    None => [glam::Vec2::ZERO; 2],
                };
                (m.env_dyn, m.prim_dyn, m.uv_dyn) = (None, None, [None; 2]);
                for (i, s) in m.textures.iter_mut().enumerate() {
                    if let Some(s) = s {
                        s.image = match slot0 {
                            Some(k) if i == 0 => k,
                            _ => tex[s.image],
                        };
                    }
                }
                uv_off.push(off);
                out.intern_material(m)
            })
            .collect();
        let projective = cmd.transform.row(3) != glam::Vec4::W;
        let mut k = 0;
        for b in &mesh.batches {
            let off = uv_off[b.material];
            let mut vertices = Vec::with_capacity(b.vertices.len());
            for v in &b.vertices {
                let m = if v.bone == NO_BONE { cmd.transform } else { cmd.transform * cmd.bones.get(v.bone as usize).copied().unwrap_or(Mat4::IDENTITY) };
                let pos = if projective { m.project_point3(v.pos) } else { m.transform_point3(v.pos) };
                let normal = root.transform_vector3(Mat3::from_mat4(m) * v.normal).normalize_or_zero();
                let color = p.vertex_colors.as_ref().and_then(|c| c.get(k)).copied().unwrap_or(v.color);
                k += 1;
                vertices.push(Vertex { bone: NO_BONE, pos: root.transform_point3(pos), normal, color, uv: [v.uv[0] + off[0], v.uv[1] + off[1]] });
            }
            out.batches.push(Batch { material: mats[b.material], vertices, sources: Vec::new() });
        }
    }
    out.stats.triangles = out.triangle_count();
    out
}

/// A mesh's bounds (its vertices as they are, bones ignored); `None` for an empty one.
pub fn bounds(draw: &DrawList) -> Option<(Vec3, Vec3)> {
    let mut it = draw.batches.iter().flat_map(|b| b.vertices.iter().map(|v| v.pos)).peekable();
    it.peek()?;
    Some(it.fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| (lo.min(p), hi.max(p))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combiner::Combiner;
    use crate::draw::{BlendMode, CullMode, DynTile, Material, SegmentValues, TextureSlot};
    use crate::submit::MeshKey;
    use crate::texture::WrapMode;

    fn material() -> Material {
        Material {
            combiner: Combiner::decode(0),
            two_cycle: false,
            prim: [1, 2, 3, 4],
            prim_lod_frac: 0,
            env: [5, 6, 7, 8],
            fog: [0; 4],
            blend_color: [0; 4],
            geometry_mode: 0,
            othermode_h: 0,
            othermode_l: 0,
            textures: [Some(TextureSlot { image: 0, wrap_s: WrapMode::Repeat, wrap_t: WrapMode::Repeat }), None],
            blend: BlendMode::Opaque,
            cull: CullMode::Back,
            depth_test: true,
            depth_write: true,
            decal: false,
            lit: false,
            texgen: false,
            bilinear: true,
            uv_dyn: [Some(DynTile { segment: 8, tile: 0, uls: 0, ult: 0, width: 32, height: 32 }), None],
            env_dyn: Some(8),
            prim_dyn: None,
            fog_blend: false,
        }
    }

    fn mesh() -> DrawList {
        let mut d = DrawList::default();
        let m = d.intern_material(material());
        d.intern_texture(7, || TextureImage { image: DecodedImage { width: 1, height: 1, rgba: vec![9, 9, 9, 255] }, fmt: 0, siz: 2, hash: 7, source_segments: 0, source_addr: Some(0x0600_0000) });
        let v = |bone: u16, x: f32| Vertex { bone, pos: Vec3::new(x, 0.0, 0.0), normal: Vec3::Y, color: [10, 20, 30, 255], uv: [glam::Vec2::ZERO; 2] };
        d.batches.push(Batch { material: m, vertices: vec![v(0, 1.0), v(1, 1.0), v(NO_BONE, 1.0)], sources: vec![] });
        d
    }

    /// Each vertex follows its bone and the draw's transform, then the root; the draw's segment
    /// colour, scroll and vertex colours are written in, and its dynamic links dropped.
    #[test]
    fn draws_are_posed_and_their_values_baked_in() {
        let m = mesh();
        let mut cmd = DrawCmd::new(MeshKey::named("m"), Mat4::from_translation(Vec3::new(0.0, 100.0, 0.0)));
        cmd.bones = vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::new(0.0, 0.0, 5.0))];
        let mut seg = SegmentValues::default();
        seg.env[8] = Some([200, 100, 50, 255]);
        seg.tiles[8][0] = Some((64, 0));
        cmd.params.segments = Some(seg);
        cmd.params.vertex_colors = Some(vec![[1, 1, 1, 1]]);
        let root = Mat4::from_translation(Vec3::new(-10.0, 0.0, 0.0));
        let f = flatten([(&cmd, &m)], root);
        let vs = &f.batches[0].vertices;
        assert_eq!(vs.iter().map(|v| v.pos).collect::<Vec<_>>(), vec![Vec3::new(-9.0, 100.0, 0.0), Vec3::new(-9.0, 100.0, 5.0), Vec3::new(-9.0, 100.0, 0.0)]);
        assert!(vs.iter().all(|v| v.bone == NO_BONE));
        assert_eq!((vs[0].color, vs[1].color), ([1, 1, 1, 1], [10, 20, 30, 255]));
        // tile origin 64 (16 texels of a 32-wide tile) scrolls the uvs back half a repeat
        assert_eq!(vs[0].uv[0], glam::Vec2::new(-0.5, 0.0));
        let mat = &f.materials[f.batches[0].material];
        assert_eq!((mat.env, mat.prim, mat.env_dyn, mat.uv_dyn[0]), ([200, 100, 50, 255], [1, 2, 3, 4], None, None));
        assert!(!mat.is_dynamic());
        assert_eq!(bounds(&f), Some((Vec3::new(-9.0, 100.0, 0.0), Vec3::new(-9.0, 100.0, 5.0))));
    }

    /// Two draws of one mesh share its texture; a draw's own texels replace the one from their
    /// source.
    #[test]
    fn textures_are_shared_and_replaced_by_source() {
        let m = mesh();
        let a = DrawCmd::new(MeshKey::named("m"), Mat4::IDENTITY);
        let mut b = a.clone();
        b.params.texture_images = vec![crate::submit::SourceImage { source: 0x0600_0000, image: DrawImage { width: 1, height: 1, rgba: vec![1u8, 2, 3, 4].into() } }];
        let f = flatten([(&a, &m), (&a, &m), (&b, &m)], Mat4::IDENTITY);
        assert_eq!(f.textures.len(), 2);
        assert_eq!(f.batches.len(), 3);
        let img = |k: usize| &f.textures[f.materials[f.batches[k].material].textures[0].unwrap().image].image.rgba;
        assert_eq!((img(0), img(2)), (&vec![9, 9, 9, 255], &vec![1, 2, 3, 4]));
    }
}
