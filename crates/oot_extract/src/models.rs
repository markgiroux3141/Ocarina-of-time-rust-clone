//! Skeletons → skinned glTF with every animation that belongs to them; standalone display
//! lists (props, items) → static glTF. Link gets all Player animations and Player's default
//! loadout (hands, sheath, shield, eyes, tunic) from the decomp's draw rules.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3};
use oot_core::anim::{JointTable, StandardAnimation};
use oot_core::gbi::DrawList;
use oot_core::model::{Binding, BuildOptions, build_draw_list};
use oot_core::player::{Age, Loadout, PlayerModel, PlayerRules, player_animations};
use oot_core::project::Project;
use oot_core::skeleton::{LimbType, Skeleton, binang_to_rad};
use oot_core::symbols::AssetFile;
use serde_json::json;

use crate::gltf::Gltf;
use crate::{asset_dir, sanitize, write_json, xml_root};

/// Actors are usually drawn at scale 0.01, so exported objects are wrapped in a node with
/// this scale to put them in game world units (the same units as scenes).
pub const ACTOR_SCALE: f32 = 0.01;

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn limb_quat(r: [i16; 3]) -> Quat {
    Quat::from_rotation_z(binang_to_rad(r[2])) * Quat::from_rotation_y(binang_to_rad(r[1])) * Quat::from_rotation_x(binang_to_rad(r[0]))
}

/// A named animation as per-frame joint tables.
pub struct NamedAnim {
    pub name: String,
    pub frames: Vec<JointTable>,
}

/// Writes a skinned glTF of `skel` with `draw` (vertices in bone-local space) and animations.
pub fn write_skinned(
    path: &Path,
    name: &str,
    skel: &Skeleton,
    limb_names: &[String],
    draw: &DrawList,
    anims: &[NamedAnim],
    root_scale: f32,
    extras: serde_json::Value,
    weights: Option<&dyn Fn(&oot_core::gbi::Vertex) -> ([u16; 4], [f32; 4])>,
) -> Result<usize> {
    let mut g = Gltf::new();
    let bind = skel.bind_pose();
    let armature = g.node(json!({ "name": name, "scale": [ACTOR_SCALE, ACTOR_SCALE, ACTOR_SCALE] }));
    let first_joint = g.nodes.len();
    for (i, limb) in skel.limbs.iter().enumerate() {
        let t = limb.joint_pos.map(|c| c as f32);
        let n = limb_names.get(i).cloned().unwrap_or_else(|| format!("limb_{i:02}"));
        g.node(json!({ "name": n, "translation": t }));
    }
    for (i, p) in skel.parents.iter().enumerate() {
        match p {
            Some(p) => g.add_child(first_joint + *p as usize, first_joint + i),
            None => g.add_child(armature, first_joint + i),
        }
    }
    let joints: Vec<usize> = (0..skel.limbs.len()).map(|i| first_joint + i).collect();
    let map: Vec<u16> = (0..skel.limbs.len() as u16).collect();
    let inverse: Vec<Mat4> = bind.iter().map(|m| m.inverse()).collect();
    let skin = g.skin(name, &joints, &inverse, first_joint);
    let mesh = match weights {
        Some(w) => g.add_draw_list_weighted(draw, name, Some(&bind), Some(w)),
        None => g.add_draw_list(draw, name, Some(&bind), Some(&map)),
    };
    if let Some(mesh) = mesh {
        let n = g.node(json!({ "name": format!("{name}_mesh"), "mesh": mesh, "skin": skin }));
        g.add_child(armature, n);
    }
    for a in anims {
        let tracks: Vec<_> = (0..skel.limbs.len())
            .map(|l| {
                let rot = Some(a.frames.iter().map(|f| limb_quat(f.rot.get(l + 1).copied().unwrap_or([0; 3]))).collect());
                let trans = (l == 0).then(|| {
                    a.frames
                        .iter()
                        .map(|f| {
                            let r = f.rot.first().copied().unwrap_or([0; 3]);
                            Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32) * root_scale
                        })
                        .collect()
                });
                (first_joint + l, trans, rot)
            })
            .collect();
        g.animation(&a.name, 20.0, a.frames.len(), &tracks);
    }
    g.extras = Some(extras);
    g.write_glb(path, &[armature])?;
    Ok(draw.triangle_count())
}

struct Keeps {
    gameplay_keep: Option<Arc<[u8]>>,
    field_keep: Option<Arc<[u8]>>,
}

fn bindings(k: &Keeps, file: &AssetFile, data: &Arc<[u8]>) -> Vec<Binding> {
    let mut b = Vec::new();
    if let Some(d) = &k.gameplay_keep {
        b.push(Binding { segment: 4, buf: d.clone(), base: 0 });
    }
    if let Some(d) = &k.field_keep {
        b.push(Binding { segment: 5, buf: d.clone(), base: 0 });
    }
    b.push(Binding { segment: file.segment.unwrap_or(6), buf: data.clone(), base: 0 });
    b
}

/// Guesses eye/mouth textures for segments 0x08/0x09 (bound by actor draw code at runtime).
/// Returns candidate bindings; callers keep them only if they don't disturb the geometry.
fn face_guesses(file: &AssetFile, data: &Arc<[u8]>) -> Vec<(Binding, String)> {
    let pick = |keys: &[&str]| {
        let texs: Vec<_> = file.of_kind("Texture").collect();
        keys.iter().find_map(|k| texs.iter().find(|t| t.name.contains(k)).map(|t| (t.offset as usize, t.name.clone())))
    };
    let mut out = Vec::new();
    if let Some((o, n)) = pick(&["EyeOpen", "EyesOpen", "Eye1", "Eyes1", "EyeTex", "Eye"]) {
        out.push((Binding { segment: 8, buf: data.clone(), base: o }, n));
    }
    if let Some((o, n)) = pick(&["MouthClosed", "Mouth1", "MouthTex", "Mouth"]) {
        out.push((Binding { segment: 9, buf: data.clone(), base: o }, n));
    }
    out
}

fn unresolved_in(d: &DrawList, seg: &str) -> usize {
    d.stats.unresolved_addresses.iter().filter(|(a, _)| a.starts_with(seg)).map(|(_, n)| n).sum()
}

/// Builds a draw list, then retries with guessed face textures if segments 08/09 are
/// unresolved; keeps the guess only if triangle and unknown-opcode counts are unchanged
/// (a wrong guess that makes the interpreter run texture bytes as a display list shows up
/// in both).
fn build_with_faces(skel: &Skeleton, mut opts: BuildOptions, file: &AssetFile, data: &Arc<[u8]>) -> Result<(DrawList, Vec<String>)> {
    let base = build_draw_list(skel, &opts)?;
    let need = unresolved_in(&base, "08") + unresolved_in(&base, "09");
    if need == 0 {
        return Ok((base, Vec::new()));
    }
    let guesses = face_guesses(file, data);
    if guesses.is_empty() {
        return Ok((base, Vec::new()));
    }
    for (b, _) in &guesses {
        opts.bindings.push(b.clone());
    }
    let tried = build_draw_list(skel, &opts)?;
    let same_geo = tried.triangle_count() == base.triangle_count();
    let unknown = |d: &DrawList| d.stats.unknown_opcodes.values().sum::<usize>();
    if same_geo && unknown(&tried) == unknown(&base) {
        Ok((tried, guesses.into_iter().map(|g| g.1).collect()))
    } else {
        Ok((base, Vec::new()))
    }
}

fn limb_names(file: &AssetFile, data: &[u8], skel_off: usize, count: usize) -> Vec<String> {
    let by_off: HashMap<u32, &str> = file.of_kind("Limb").map(|s| (s.offset, s.name.as_str())).collect();
    let table = (be32(data, skel_off) & 0xFF_FFFF) as usize;
    (0..count)
        .map(|i| {
            let o = table + i * 4;
            let lo = if o + 4 <= data.len() { be32(data, o) & 0xFF_FFFF } else { 0 };
            by_off.get(&lo).map(|s| s.to_string()).unwrap_or_else(|| format!("limb_{i:02}"))
        })
        .collect()
}

/// Joint count implied by the layout ZAPD/the original tools use: the joint index table sits
/// directly before the header.
fn anim_joint_count(data: &[u8], seg: u8, off: usize) -> Option<usize> {
    if off + 12 > data.len() {
        return None;
    }
    let idx = be32(data, off + 8);
    if (idx >> 24) as u8 != seg {
        return None;
    }
    let idx = (idx & 0xFF_FFFF) as usize;
    (idx < off && (off - idx) % 6 == 0).then(|| (off - idx) / 6)
}

/// Skeleton limb for one of Link's extra display lists, from its name.
fn link_prop_limb(name: &str) -> Option<u16> {
    const RULES: &[(&str, u16)] = &[
        ("LeftIronBoot", 8), ("LeftHoverBoot", 8), ("RightIronBoot", 5), ("RightHoverBoot", 5),
        ("LeftGauntlet", 15), ("RightGauntlet", 18), ("LeftArmOut", 14), ("RightArmOut", 17),
        ("LeftHand", 15), ("LeftFist", 15), ("HandHoldingBrokenGiantsKnife", 15), ("HandHoldingBottle", 15),
        ("RightHand", 18), ("RightFist", 18), ("RightArmStretched", 17),
        ("Sheath", 19), ("DekuShieldWithMatrix", 19), ("Waist", 1), ("LeftShoulder", 13), ("RightShoulder", 16),
        ("LeftArm", 14), ("RightArm", 17), ("LeftThigh", 6), ("RightThigh", 3), ("LeftLeg", 7), ("RightLeg", 4),
        ("LeftFoot", 8), ("RightFoot", 5), ("Head", 10), ("Hat", 11), ("Collar", 12), ("Torso", 20),
        ("GoronBracelet", 15),
    ];
    RULES.iter().find(|(k, _)| name.contains(k)).map(|r| r.1)
}

/// Link's extra display lists (hand poses, swords, shields, boots, gauntlets, masks, items)
/// as static glbs in the skeleton's bind-pose space, so they line up with the skinned model.
/// Items drawn with their own matrix at runtime (bottle, bow string, hookshot parts) are
/// left in local space.
fn link_props(model: &PlayerModel, rules: &PlayerRules, lo: &Loadout, dir: &Path, tris: &mut usize, errors: &mut Vec<String>) -> Result<usize> {
    let skel = &model.skeleton;
    let bind = skel.bind_pose();
    let limb_dls: HashSet<u32> = skel.limbs.iter().flat_map(|l| l.dlists.iter().copied()).collect();
    let tunic = rules.tunics.get(lo.tunic).map(|t| t.1).unwrap_or([30, 105, 27]);
    let mut n = 0;
    let mut index = Vec::new();
    for d in model.symbols.of_kind("DList") {
        let addr = 0x0600_0000 | d.offset;
        if limb_dls.contains(&addr) {
            continue;
        }
        let limb = link_prop_limb(&d.name);
        let mut it = oot_core::gbi::Interpreter::new();
        use oot_core::gbi::Segment;
        it.segments[4] = Some(Segment::Data { buf: model.gameplay_keep.clone(), base: 0 });
        it.segments[6] = Some(Segment::Data { buf: model.object.clone(), base: 0 });
        it.segments[8] = Some(Segment::Data { buf: model.object.clone(), base: model.eye_offsets[0] });
        it.segments[9] = Some(Segment::Data { buf: model.object.clone(), base: model.mouth_offsets[0] });
        it.segments[0x0C] = Some(oot_core::model::cull_back_builtin());
        it.segments[0x0D] = Some(Segment::Matrices(skel.flex_matrix_map(0)));
        it.apply_setup_dl_25();
        it.set_env_color([tunic[0], tunic[1], tunic[2], 0]);
        it.set_bone(limb.unwrap_or(oot_core::gbi::NO_BONE));
        it.run(addr);
        let draw = it.draw;
        if draw.triangle_count() == 0 {
            continue;
        }
        let mut g = Gltf::new();
        let Some(mesh) = g.add_draw_list(&draw, &d.name, Some(&bind), None) else { continue };
        let node = g.node(json!({ "name": d.name, "mesh": mesh, "scale": [ACTOR_SCALE, ACTOR_SCALE, ACTOR_SCALE] }));
        g.extras = Some(json!({
            "source": model.age.object(),
            "dlist": d.name,
            "space": if limb.is_some() { "Link's bind pose (lines up with the skinned model's rest pose)" } else { "local (positioned by code at runtime)" },
            "limb": limb,
        }));
        let path = dir.join(format!("{}.glb", sanitize(&d.name)));
        match g.write_glb(&path, &[node]) {
            Ok(()) => {
                n += 1;
                *tris += draw.triangle_count();
                index.push(json!({ "dlist": d.name, "glb": format!("dlists/{}.glb", sanitize(&d.name)), "limb": limb, "triangles": draw.triangle_count() }));
            }
            Err(e) => errors.push(format!("{} / {}: {e:#}", model.age.object(), d.name)),
        }
    }
    write_json(&dir.parent().unwrap_or(dir).join("models.json"), &json!({ "file": model.age.object(), "units": "node scale 0.01 = game world units", "props": index }))?;
    Ok(n)
}

fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}

/// Skin skeletons (horses): limbs are either rigid display lists (type 11) or "animated"
/// limbs (type 4) whose vertices are weighted blends of several limb matrices
/// (`Skin_ApplyLimbModifications`). Builds the vertex buffers at the bind pose, runs the
/// display lists over them, and exports real multi-weight glTF skinning.
/// Returns (triangles, animations, display-list addresses used).
fn export_skin(keeps: &Keeps, f: &AssetFile, data: &Arc<[u8]>, seg: u8, sym: &oot_core::symbols::Symbol, dir: &Path) -> Result<(usize, usize, Vec<u32>)> {
    use oot_core::gbi::{Interpreter, NO_BONE, Segment};
    let local = |addr: u32| -> Result<usize> {
        anyhow::ensure!((addr >> 24) as u8 == seg, "pointer {addr:08X} not in segment {seg:02X}");
        let o = (addr & 0xFF_FFFF) as usize;
        anyhow::ensure!(o < data.len(), "pointer {addr:08X} outside file");
        Ok(o)
    };
    // SkinLimb has jointPos/child/sibling like other limbs, then segmentType and segment,
    // which the LOD layout reads as its two display-list words.
    let skel = Skeleton::parse(data, seg, sym.offset as usize, LimbType::Lod, false)?;
    let bind = skel.bind_pose();
    let mat = |l: usize| bind.get(l).copied().unwrap_or(Mat4::IDENTITY);
    let mut it = Interpreter::new();
    for b in bindings(keeps, f, data) {
        it.segments[b.segment as usize & 0xF] = Some(Segment::Data { buf: b.buf, base: b.base });
    }
    it.segments[0x0C] = Some(oot_core::model::cull_back_builtin());
    it.apply_setup_dl_25();
    let mut weights: HashMap<[i32; 3], ([u16; 4], [f32; 4])> = HashMap::new();
    let mut used = Vec::new();
    for &limb in &skel.draw_order {
        let (ty, ptr) = (skel.limbs[limb as usize].dlists[0], skel.limbs[limb as usize].dlists[1]);
        if ptr == 0 {
            continue;
        }
        match ty {
            11 => {
                it.set_bone(limb as u16);
                it.run(ptr);
                used.push(ptr);
            }
            4 => {
                let d = local(ptr)?;
                let total = be16(data, d) as usize;
                let modif_count = be16(data, d + 2) as usize;
                let modifs = local(be32(data, d + 4))?;
                let dl = be32(data, d + 8);
                let mut buf = vec![0u8; total * 16];
                for m in 0..modif_count {
                    let mo = modifs + m * 16;
                    let (vcount, tcount, main) = (be16(data, mo) as usize, be16(data, mo + 2) as usize, be16(data, mo + 4) as usize);
                    if tcount == 0 {
                        continue;
                    }
                    let sv = local(be32(data, mo + 8))?;
                    let lt = local(be32(data, mo + 12))?;
                    let tr: Vec<(usize, Vec3, f32)> = (0..tcount)
                        .map(|t| {
                            let o = lt + t * 10;
                            let v = Vec3::new(be16(data, o + 2) as i16 as f32, be16(data, o + 4) as i16 as f32, be16(data, o + 6) as i16 as f32);
                            (data[o] as usize, v, data[o + 8] as f32 * 0.01)
                        })
                        .collect();
                    let (pos, mut w): (Vec3, Vec<(usize, f32)>) = if tcount == 1 {
                        (mat(tr[0].0).transform_point3(tr[0].1), vec![(tr[0].0, 1.0)])
                    } else {
                        (tr.iter().map(|t| mat(t.0).transform_point3(t.1) * t.2).sum(), tr.iter().map(|t| (t.0, t.2)).collect())
                    };
                    w.sort_by(|a, b| b.1.total_cmp(&a.1));
                    w.truncate(4);
                    let sum: f32 = w.iter().map(|x| x.1).sum::<f32>().max(1e-6);
                    let mut j = [0u16; 4];
                    let mut ww = [0f32; 4];
                    for (k, (l, x)) in w.iter().enumerate() {
                        j[k] = *l as u16;
                        ww[k] = x / sum;
                    }
                    let ipos = [pos.x.round() as i32, pos.y.round() as i32, pos.z.round() as i32];
                    weights.insert(ipos, (j, ww));
                    let nmat = mat(tr.get(main).map_or(tr[0].0, |t| t.0));
                    for k in 0..vcount {
                        let o = sv + k * 10;
                        let idx = be16(data, o) as usize;
                        if idx >= total {
                            continue;
                        }
                        let n = Vec3::new(data[o + 6] as i8 as f32, data[o + 7] as i8 as f32, data[o + 8] as i8 as f32);
                        let n = nmat.transform_vector3(n).clamp(Vec3::splat(-127.0), Vec3::splat(127.0));
                        let v = &mut buf[idx * 16..idx * 16 + 16];
                        for (c, val) in ipos.iter().enumerate() {
                            v[c * 2..c * 2 + 2].copy_from_slice(&(*val as i16).to_be_bytes());
                        }
                        v[8..10].copy_from_slice(&data[o + 2..o + 4]);
                        v[10..12].copy_from_slice(&data[o + 4..o + 6]);
                        v[12] = n.x as i8 as u8;
                        v[13] = n.y as i8 as u8;
                        v[14] = n.z as i8 as u8;
                        v[15] = data[o + 9];
                    }
                }
                it.segments[8] = Some(Segment::Data { buf: buf.into(), base: 0 });
                it.set_bone(NO_BONE);
                it.run(dl);
                used.push(dl);
            }
            _ => {}
        }
    }
    let draw = it.draw;
    let wf = |v: &oot_core::gbi::Vertex| -> ([u16; 4], [f32; 4]) {
        if v.bone != NO_BONE {
            return ([v.bone, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]);
        }
        let k = [v.pos.x.round() as i32, v.pos.y.round() as i32, v.pos.z.round() as i32];
        weights.get(&k).copied().unwrap_or(([0; 4], [1.0, 0.0, 0.0, 0.0]))
    };
    let joints = skel.limbs.len() + 1;
    let anims: Vec<NamedAnim> = f
        .of_kind("Animation")
        .filter(|a| anim_joint_count(data, seg, a.offset as usize).is_none_or(|n| n == joints))
        .filter_map(|a| StandardAnimation::parse(data, seg, a.offset as usize, skel.limbs.len()).ok().map(|sa| NamedAnim { name: a.name.clone(), frames: sa.frames }))
        .collect();
    let names = limb_names(f, data, sym.offset as usize, skel.limbs.len());
    let path = dir.join(format!("{}.glb", sanitize(&sym.name)));
    let extras = json!({ "source": f.name, "skeleton": sym.name, "skin": true, "note": "Skin skeleton: weighted vertices from SkinLimbModif data, up to 4 weights per vertex" });
    let t = write_skinned(&path, &sym.name, &skel, &names, &draw, &anims, 1.0, extras, Some(&wf))?;
    Ok((t, anims.len(), used))
}

pub fn extract(p: &Project, out: &Path) -> Result<serde_json::Value> {
    let root = xml_root(p);
    let keeps = Keeps { gameplay_keep: p.rom.file_by_name("gameplay_keep").ok(), field_keep: p.rom.file_by_name("gameplay_field_keep").ok() };
    let rules = PlayerRules::load(&p.config.decomp)?;
    let (mut skels_ok, mut skels_skipped, mut anims_total, mut statics, mut tris) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut face_guessed = 0usize;
    let mut skipped = Vec::new();
    let mut errors = Vec::new();

    // Link: both ages with Player's default loadout and every Player animation.
    let player_anims: Vec<NamedAnim> = player_animations(p)?
        .into_iter()
        .map(|(n, a)| NamedAnim { name: n.strip_prefix("gPlayerAnim_").unwrap_or(&n).to_string(), frames: a.frames })
        .collect();
    for age in [Age::Adult, Age::Child] {
        let model = PlayerModel::load(p, &rules, age)?;
        let lo = Loadout::default_for(&rules, age);
        let (draw, _) = model.draw_list(&rules, &lo, 0, 0, 0)?;
        let skel_sym = model.symbols.of_kind("Skeleton").next().context("Link skeleton")?;
        let names = limb_names(&model.symbols, &model.object, skel_sym.offset as usize, model.skeleton.limbs.len());
        let dir = asset_dir(out, "models", &model.symbols, &root);
        let path = dir.join(format!("{}.glb", sanitize(&skel_sym.name)));
        tris += write_skinned(
            &path,
            &skel_sym.name,
            &model.skeleton,
            &names,
            &draw,
            &player_anims,
            age.root_scale(),
            json!({
                "source": age.object(),
                "loadout": "PLAYER_MODELGROUP_DEFAULT, default shield, Kokiri tunic, eyes open",
                "animations": "all Player animations (gameplay_keep headers, link_animetion data), 20 fps",
                "root_translation_scale": age.root_scale(),
            }),
            None,
        )?;
        anims_total += player_anims.len();
        skels_ok += 1;
        statics += link_props(&model, &rules, &lo, &dir.join("dlists"), &mut tris, &mut errors)?;
    }

    for f in &p.symbols.files {
        if f.name == "object_link_boy" || f.name == "object_link_child" {
            continue;
        }
        let is_scene = matches!(f.segment, Some(2) | Some(3));
        let is_overlay = f.name.starts_with("ovl_");
        let skel_syms: Vec<_> = f.of_kind("Skeleton").collect();
        let dl_syms: Vec<_> = f.of_kind("DList").collect();
        if (skel_syms.is_empty() && dl_syms.is_empty()) || is_scene || is_overlay {
            continue;
        }
        let Ok(data) = p.rom.file_by_name(&f.name) else { continue };
        let seg = f.segment.unwrap_or(6);
        let dir = asset_dir(out, "models", f, &root);
        let mut limb_dls: HashSet<u32> = HashSet::new();
        let mut file_index = Vec::new();

        for s in &skel_syms {
            let lt = match s.attr("LimbType") {
                Some("Standard") => LimbType::Standard,
                Some("LOD") => LimbType::Lod,
                Some("Skin") => {
                    match export_skin(&keeps, f, &data, seg, s, &dir) {
                        Ok((t, n_anims, limbs)) => {
                            tris += t;
                            skels_ok += 1;
                            anims_total += n_anims;
                            limb_dls.extend(limbs);
                            file_index.push(json!({ "skeleton": s.name, "glb": format!("{}.glb", sanitize(&s.name)), "skin": true, "triangles": t, "animations": n_anims }));
                        }
                        Err(e) => errors.push(format!("{} / {}: {e:#}", f.name, s.name)),
                    }
                    continue;
                }
                other => {
                    skels_skipped += 1;
                    skipped.push(format!("{} / {}: limb type {:?} not supported", f.name, s.name, other));
                    continue;
                }
            };
            let flex = s.attr("Type") == Some("Flex");
            let parsed = Skeleton::parse(&data, seg, s.offset as usize, lt, flex).and_then(|sk| {
                // Some XMLs label flex skeletons as normal; the header's dListCount gives it away.
                let o = s.offset as usize + 8;
                if !flex && data.get(o).is_some_and(|&n| n != 0 && n as usize == sk.flex_matrix_map(0).len()) {
                    Skeleton::parse(&data, seg, s.offset as usize, lt, true)
                } else {
                    Ok(sk)
                }
            });
            let skel = match parsed {
                Ok(sk) => sk,
                Err(e) => {
                    errors.push(format!("{} / {}: {e:#}", f.name, s.name));
                    continue;
                }
            };
            for l in &skel.limbs {
                limb_dls.extend(l.dlists.iter().copied().filter(|&d| d != 0));
            }
            let opts = BuildOptions { bindings: bindings(&keeps, f, &data), ..Default::default() };
            let (draw, guessed) = match build_with_faces(&skel, opts, f, &data) {
                Ok(x) => x,
                Err(e) => {
                    errors.push(format!("{} / {}: {e:#}", f.name, s.name));
                    continue;
                }
            };
            face_guessed += !guessed.is_empty() as usize;
            let joints = skel.limbs.len() + 1;
            let anims: Vec<NamedAnim> = f
                .of_kind("Animation")
                .filter(|a| anim_joint_count(&data, seg, a.offset as usize).is_none_or(|n| n == joints) || skel_syms.len() == 1)
                .filter_map(|a| {
                    StandardAnimation::parse(&data, seg, a.offset as usize, skel.limbs.len())
                        .ok()
                        .map(|sa| NamedAnim { name: a.name.clone(), frames: sa.frames })
                })
                .collect();
            let names = limb_names(f, &data, s.offset as usize, skel.limbs.len());
            let unresolved: HashMap<String, usize> = draw.stats.unresolved_addresses.iter().fold(HashMap::new(), |mut m, (a, n)| {
                *m.entry(format!("seg {}", &a[..2])).or_default() += n;
                m
            });
            let path = dir.join(format!("{}.glb", sanitize(&s.name)));
            let extras = json!({
                "source": f.name,
                "skeleton": s.name,
                "flex": skel.flex,
                "guessed_face_textures": guessed,
                "unresolved_references": unresolved,
                "note": "Segments 08-0D are bound by actor draw code at runtime; unresolved ones leave parts untextured or missing.",
            });
            match write_skinned(&path, &s.name, &skel, &names, &draw, &anims, 1.0, extras, None) {
                Ok(t) => {
                    tris += t;
                    skels_ok += 1;
                    anims_total += anims.len();
                    file_index.push(json!({ "skeleton": s.name, "glb": path.file_name().map(|n| n.to_string_lossy().to_string()), "limbs": skel.limbs.len(), "triangles": t, "animations": anims.iter().map(|a| &a.name).collect::<Vec<_>>(), "unresolved": unresolved, "guessed_face_textures": guessed }));
                }
                Err(e) => errors.push(format!("{} / {}: {e:#}", f.name, s.name)),
            }
        }

        // Display lists that aren't limbs of a skeleton in this file: props, items, parts.
        for d in dl_syms {
            let addr = ((seg as u32) << 24) | d.offset;
            if limb_dls.contains(&addr) {
                continue;
            }
            let mut it = oot_core::gbi::Interpreter::new();
            for b in bindings(&keeps, f, &data) {
                it.segments[b.segment as usize & 0xF] = Some(oot_core::gbi::Segment::Data { buf: b.buf, base: b.base });
            }
            it.segments[0x0C] = Some(oot_core::model::cull_back_builtin());
            it.apply_setup_dl_25();
            it.run(addr);
            let draw = it.draw;
            if draw.triangle_count() == 0 {
                continue;
            }
            let mut g = Gltf::new();
            let Some(mesh) = g.add_draw_list(&draw, &d.name, None, None) else { continue };
            let node = g.node(json!({ "name": d.name, "mesh": mesh, "scale": [ACTOR_SCALE, ACTOR_SCALE, ACTOR_SCALE] }));
            g.extras = Some(json!({ "source": f.name, "dlist": d.name, "offset": format!("0x{:X}", d.offset) }));
            let path = dir.join("dlists").join(format!("{}.glb", sanitize(&d.name)));
            match g.write_glb(&path, &[node]) {
                Ok(()) => {
                    statics += 1;
                    tris += draw.triangle_count();
                    file_index.push(json!({ "dlist": d.name, "glb": format!("dlists/{}.glb", sanitize(&d.name)), "triangles": draw.triangle_count() }));
                }
                Err(e) => errors.push(format!("{} / {}: {e:#}", f.name, d.name)),
            }
        }
        if !file_index.is_empty() {
            write_json(&dir.join("models.json"), &json!({ "file": f.name, "units": "node scale 0.01 = game world units", "models": file_index }))?;
        }
    }
    errors.truncate(50);
    Ok(json!({
        "skeletons": skels_ok,
        "skeletons_skipped": skels_skipped,
        "skipped": skipped,
        "animations": anims_total,
        "static_dlists": statics,
        "triangles": tris,
        "face_textures_guessed": face_guessed,
        "errors": errors,
    }))
}
