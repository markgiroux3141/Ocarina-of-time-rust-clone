//! `oot capture-actors` (`oot::capture`): what each actor's own draw code submits, captured for the
//! game editor's views (docs/EDITOR-ACTORS.md, "Models in the views"; ADR 0078).

use std::path::{Path, PathBuf};

use oot::capture::{CaptureOptions, capture_actors};
use oot_game::actor_capture::{ActorCaptures, Manifest, Outcome, actor_file};

fn pack_is_current() -> bool {
    oot_game::pack::default_pack_path().is_ok_and(|p| oot_game::pack::is_current(&p, None))
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// Pots, signs, Kokiri children and Navi's call spots: captured standing at the origin at the
/// game's size, the Kokiri only as if in their own scenes (their code checks it), the spots not at
/// all (they draw nothing); a second run keeps every file as it was.
#[test]
fn actors_are_captured_as_their_own_draw_code_draws_them() {
    if !pack_is_current() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("oot_actor_captures_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let o = CaptureOptions { out: dir.clone(), root: root(), all: false, only: ["Obj_Tsubo", "En_Kanban", "En_Ko", "Elf_Msg"].map(String::from).to_vec(), pngs: false };
    capture_actors(&o).expect("the capture");
    let m = Manifest::read(&dir).expect("the manifest");
    let find = |actor: &str, params: u16| m.entries.iter().find(|e| e.actor == actor && e.params == params && e.rot == [0; 3]).unwrap_or_else(|| panic!("{actor} {params:#x} tried"));

    // a pot as the game draws it: some 23 across, standing on the ground at the origin
    let pot = find("Obj_Tsubo", 0);
    assert_eq!(pot.outcome, Outcome::Captured, "{}", pot.note);
    assert!((15.0..40.0).contains(&pot.size[0]) && (15.0..40.0).contains(&pot.size[1]), "{:?}", pot.size);
    let pots = ActorCaptures::read(&dir, "Obj_Tsubo").expect("the pots' file");
    let c = pots.get(0, [0; 3]).expect("params 0");
    assert!(c.lo[1].abs() < 3.0 && (c.lo[0] + c.hi[0]).abs() < 4.0 && (c.lo[2] + c.hi[2]).abs() < 4.0, "on the ground round the origin: {:?} {:?}", c.lo, c.hi);
    assert!(c.mesh.batches.iter().all(|b| b.vertices.iter().all(|v| v.bone == eng_gfx::NO_BONE)), "flattened");
    assert!(!c.mesh.textures.is_empty(), "textured");
    // every configuration the game places it with is tried, and a sign's too
    assert!(m.entries.iter().filter(|e| e.actor == "Obj_Tsubo").count() > 10);
    assert_eq!(find("En_Kanban", 0).outcome, Outcome::Captured);

    // Kokiri children hide outside Kokiri Forest and its houses: caught as if there
    let ko: Vec<_> = m.entries.iter().filter(|e| e.actor == "En_Ko" && e.outcome == Outcome::Captured).collect();
    assert!(ko.len() > 5 && ko.iter().all(|e| e.note.starts_with("as in kokiri") || e.note.starts_with("as in spot04")), "{:?}", ko.iter().map(|e| &e.note).collect::<Vec<_>>());

    // Navi's call spots draw nothing
    assert!(m.entries.iter().filter(|e| e.actor == "Elf_Msg").all(|e| e.outcome == Outcome::Nothing));

    // made again only when stale: a second run keeps the files
    let when = |a: &str| std::fs::metadata(actor_file(&dir, a)).and_then(|m| m.modified()).unwrap();
    let before = when("Obj_Tsubo");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    capture_actors(&o).expect("the second run");
    assert_eq!(when("Obj_Tsubo"), before);
    assert_eq!(Manifest::read(&dir).unwrap().entries.len(), m.entries.len());
    let _ = std::fs::remove_dir_all(&dir);
}
