"""Render regression for the spike behaviour: runs every headless case, hashes the output,
and records or checks the hashes.

    python scripts/golden.py record [--bin-dir target/<you>/release]
    python scripts/golden.py check  [--bin-dir target/<you>/release] [--only substr]

Images and traces are written to the git-ignored out/golden/ and stay local. Only the hashes
(golden/renders.sha256) are committed. PNGs are hashed on their decoded RGBA pixels and size,
so encoder settings don't matter; JSON traces and SRAM images ({sram}) are hashed as bytes.

The sandbox binary is `oot_sandbox` (spike 03-04's `oot_play` before the restructure); pass
--sandbox oot_play to run the old one.
"""

import argparse
import concurrent.futures as cf
import hashlib
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "out" / "golden"
HASHES = ROOT / "golden" / "renders.sha256"

# (case name, program, args). {sheet}, {trace}, {shot} expand to paths under out/golden/<case>.
S, V = "sandbox", "viewer"
CASES = [
    # Spike 03 course scripts.
    *[(f"course_{s}", S, ["--script", s, "--sheet", "{sheet}", "--trace", "{trace}"])
      for s in ["run-roll", "ledge", "pit", "stairs", "walls", "turn", "idle"]],
    ("course_run-roll_follow", S, ["--script", "run-roll", "--follow-camera", "--sheet", "{sheet}"]),
    ("course_run-roll_child", S, ["--script", "run-roll", "--child", "--sheet", "{sheet}", "--trace", "{trace}"]),
    ("course_wire", S, ["--script", "still", "--wire", "--screenshot", "{shot}"]),
    # Spike 04 milestones 3-7 on the course.
    *[(f"course_{s}", S, ["--script", s, "--sheet", "{sheet}", "--trace", "{trace}"])
      for s in ["climb50", "climb70", "climb100", "hang", "target", "parallel", "sword"]],
    ("course_hang_side", S, ["--script", "hang", "--view=-540,160,-420,-540,120,-700", "--sheet", "{sheet}"]),
    ("course_sword_child", S, ["--script", "sword", "--child", "--sheet", "{sheet}", "--trace", "{trace}"]),
    ("course_swim", S, ["--script", "swim", "--step", "8", "--sheet", "{sheet}", "--trace", "{trace}"]),
    ("course_platform", S, ["--script", "platform", "--step", "4", "--sheet", "{sheet}", "--trace", "{trace}"]),
    ("course_ik_on", S, ["--script", "ramp-stand", "--screenshot", "{shot}", "--trace", "{trace}"]),
    ("course_ik_off", S, ["--script", "ramp-stand", "--no-foot-ik", "--screenshot", "{shot}", "--trace", "{trace}"]),
    ("course_target_locked", S, ["--script", "target", "--frames", "10", "--screenshot", "{shot}"]),
    # Spike 04 milestone 1-2: Kokiri Forest and other scenes.
    ("spot04_forward", S, ["--scene", "spot04", "--script", "forward", "--sheet", "{sheet}", "--trace", "{trace}"]),
    ("spot04_tour", S, ["--scene", "spot04", "--script", "tour", "--sheet", "{sheet}", "--trace", "{trace}"]),
    ("spot04_child_forward", S, ["--scene", "spot04", "--child", "--script", "forward", "--sheet", "{sheet}", "--trace", "{trace}"]),
    ("spot04_tread", S, ["--scene", "spot04", "--child", "--at", "1230,-12,-130,0", "--script", "tread", "--step", "10",
                         "--sheet", "{sheet}", "--trace", "{trace}"]),
    *[(f"spot04_spawn{i}", S, ["--scene", "spot04", "--spawn", str(i), "--script", "still", "--width", "640", "--height", "360",
                               "--screenshot", "{shot}"]) for i in range(12)],
    ("spot04_follow_spawn0", S, ["--scene", "spot04", "--follow-camera", "--script", "still", "--screenshot", "{shot}"]),
    ("spot04_evening_target", S, ["--scene", "spot04", "--time", "19:00", "--target", "200", "--script", "still", "--screenshot", "{shot}"]),
    ("spot04_dawn", S, ["--scene", "spot04", "--time", "07:00", "--script", "still", "--screenshot", "{shot}"]),
    ("spot04_water_f8", S, ["--scene", "spot04", "--script", "idle", "--frames", "8", "--screenshot", "{shot}"]),
    ("spot04_water_f12", S, ["--scene", "spot04", "--script", "idle", "--frames", "12", "--screenshot", "{shot}"]),
    ("spot04_collision", S, ["--scene", "spot04", "--collision", "--script", "still", "--screenshot", "{shot}"]),
    ("spot04_view", S, ["--scene", "spot04", "--view", "400,300,1200,0,0,0", "--script", "still", "--screenshot", "{shot}"]),
    ("spot00_adult", S, ["--scene", "spot00", "--script", "still", "--screenshot", "{shot}"]),
    ("ydan_adult", S, ["--scene", "ydan", "--script", "still", "--screenshot", "{shot}"]),
    # GAME-02 milestone 4: the Deku Tree's mouth held open (EVENTCHKINF_05), and the Kokiri Forest
    # playthrough from Link's bed into the Deku Tree (entered by Play_Init: the HUD, the texts).
    ("spot04_treemouth_open", S, ["--scene", "spot04", "--spawn", "1", "--entrance", "--child", "--preset", "deku-tree-open",
                                  "--script", "idle", "--frames", "40", "--width", "640", "--height", "360", "--screenshot", "{shot}"]),
    ("playthrough", S, ["--entrance", "ENTR_LINKS_HOUSE_0", "--child", "--preset", "deku-tree-open", "--script", "playthrough",
                        "--trace", "{trace}"]),
    # GAME-03 milestone 2: a new save from Link's bed through the crawlspace, past the boulder,
    # to the Kokiri Sword's chest, opened.
    ("sword_chest", S, ["--entrance", "ENTR_LINKS_HOUSE_0", "--child", "--script", "sword-chest", "--trace", "{trace}"]),
    # GAME-03 milestone 3: a new save from Link's bed to the sword, 42 rupees, the Deku Shield
    # bought in the Kokiri shop, both worn, Mido's talk, and past him.
    ("mido_shop", S, ["--entrance", "ENTR_LINKS_HOUSE_0", "--child", "--script", "mido-shop", "--trace", "{trace}"]),
    # GAME-05 milestone 2: a Deku Baba's bite, then its stem cut, inside the Deku Tree.
    ("deku_baba", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-inside", "--script", "deku-baba", "--trace", "{trace}"]),
    # GAME-05 milestone 3a: a withered Deku Baba slashed and its stick taken, a Keese's dive
    # blocked with the shield and the Keese slashed, with the battle camera, inside the Deku Tree.
    ("combat", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-inside", "--script", "combat", "--trace", "{trace}"]),
    # GAME-05 milestone 3b: from a debug start in room 4, a Mad Scrub's nut bounced back off the
    # Deku Shield knocks it out of its flower, and it's caught and slashed, inside the Deku Tree.
    ("scrub", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-inside", "--script", "scrub", "--trace", "{trace}"]),
    # GAME-05 milestone 4a: from a debug start on room 0's top floor, the floor switch burns the
    # web over room 10's door, and Link goes through the sliding door, which bars behind him.
    ("shutter", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-inside", "--script", "shutter", "--trace", "{trace}"]),
    # GAME-05 milestone 4b: from a debug start by room 0's middle-floor golden torch (ten Deku
    # Sticks on C-Left, the torches lit), a stick lit at the torch burns the web over room 1's
    # door, and Link goes through the door into room 1.
    ("stick", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-sticks", "--script", "stick", "--trace", "{trace}"]),
    # GAME-05 milestone 4c: from a debug start on room 3's upper floor behind its push block, Link
    # pushes the block along the floor's channel and off its end into the pit (flag 0x10, the
    # chime), then goes down into the pit beside it and climbs onto it.
    ("push", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-inside", "--script", "push", "--trace", "{trace}"]),
    # GAME-05 milestone 5a: from a debug start in room 1 250 in front of its eye switch (the
    # slingshot on C-Right), Link takes the Fairy Slingshot out, aims in first person and shoots a
    # seed into the eye (flag 0x0C): the door to room 2 unbars, and he goes through it.
    ("slingshot", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-slingshot", "--script", "slingshot", "--trace", "{trace}"]),
    # GAME-05 milestone 5b-1: inside the Deku Tree with the slingshot owned but on no button,
    # Start opens the pause menu, the item page's cursor goes to the slingshot, C-Right equips it
    # (its icon flying to the button), R turns to the map page, Start closes the menu: the
    # trace and the game resumed with the slingshot on C-Right; then the run cut at frame 60
    # (the item page, the icon in flight) and at 82 (the map page).
    ("pause", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-slingshot-owned", "--script", "pause", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("pause_item", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-slingshot-owned", "--script", "pause", "--frames", "60", "--screenshot", "{shot}"]),
    ("pause_map", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-slingshot-owned", "--script", "pause", "--frames", "82", "--screenshot", "{shot}"]),
    # GAME-05 milestone 5b-2: inside the Deku Tree with the compass and 3F to 1F visited, Start, R
    # to the dungeon map page, the stick right onto the floors and up to 2F, Start: the trace and
    # the game resumed; then the run cut at frame 70 (1F: Link's head, room 0 pulsing, chest 3's
    # mark) and at 72 (2F, chest 1's mark).
    ("dungeon_map", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-compass", "--script", "dungeon-map", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("dungeon_map_1f", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-compass", "--script", "dungeon-map", "--frames", "70", "--screenshot", "{shot}"]),
    ("dungeon_map_2f", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-compass", "--script", "dungeon-map", "--frames", "72", "--screenshot", "{shot}"]),
    # GAME-05 milestone 5b-2: inside the Deku Tree with a quarter heart, the top floor's Deku
    # Baba's bite kills Link: the game over, No at "Would you like to save?", Yes at "Continue
    # playing?", the respawn: the trace and the game resumed; then the run cut at frame 170
    # ("GAME OVER" drawn), 191 (the save prompt) and 193 (the continue prompt).
    ("game_over", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-quarter-heart", "--script", "game-over", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("game_over_message", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-quarter-heart", "--script", "game-over", "--frames", "170", "--screenshot", "{shot}"]),
    ("game_over_save", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-quarter-heart", "--script", "game-over", "--frames", "191", "--screenshot", "{shot}"]),
    ("game_over_continue", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-quarter-heart", "--script", "game-over", "--frames", "193", "--screenshot", "{shot}"]),
    # GAME-05 milestone 5c: inside the Deku Tree, the slingshot onto C-Left from the pause menu,
    # B's save prompt, "Yes" (the save written to file 2 of an SRAM in memory, the menu closed),
    # then the console's reset loading file 2 back; the final SRAM image hashed ({sram}). The
    # prompt halfway in (69) and waiting on Yes (73).
    ("save", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-save", "--script", "save", "--trace", "{trace}", "--screenshot", "{shot}", "--sram", "{sram}"]),
    ("save_prompt_turn", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-save", "--script", "save", "--frames", "69", "--screenshot", "{shot}"]),
    ("save_prompt", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-save", "--script", "save", "--frames", "73", "--screenshot", "{shot}"]),
    # GAME-03 milestone 4: a new save on past Mido into the meadow, the Deku Tree's talk
    # (cutscenes gDekuTreeMeetingCs, gDekuTreeMouthOpeningCs) answered yes, and into his mouth and the Deku Tree's
    # intro (gDekuTreeIntroCs). No preset.
    ("new_save_deku_tree", S, ["--entrance", "ENTR_LINKS_HOUSE_0", "--child", "--script", "new-save-deku-tree", "--trace", "{trace}"]),
    # GAME-03 milestone 5, Phase 4's exit: the file select's new file from its first frame, the
    # opening's four scripts (Link's house's layer 5, the nightmare, Navi sent, the wake-up),
    # then the new save's run with C-Up to Navi, into the Deku Tree.
    ("new_file_deku_tree", S, ["--new-file", "--child", "--script", "new-file-deku-tree", "--trace", "{trace}"]),
    # GAME-04 milestone 3, Phase 5's exit: the Mido and shop run's audio log (the game's sound
    # effect requests with where they are, its sequence commands and the library commands),
    # the audio offline beside it.
    ("mido_shop_audio", S, ["--entrance", "ENTR_LINKS_HOUSE_0", "--child", "--script", "mido-shop", "--audio-log", "{trace}"]),
    # Spike 02 viewer.
    ("viewer_link_sheet", V, ["--sheet", "{shot}"]),
    ("viewer_link_sheet_child", V, ["--age", "child", "--sheet", "{shot}"]),
    ("viewer_link_sword", V, ["--sheet", "{shot}", "--group", "SWORD_AND_SHIELD", "--shield", "HYLIAN",
                              "--anims", "link_fighter_normal_kiru,link_fighter_defense_wait"]),
    ("viewer_link_walk6", V, ["--screenshot", "{shot}", "--anim", "link_normal_walk", "--frame", "6"]),
    ("viewer_tock_sheet", V, ["--subject", "tock", "--sheet", "{shot}"]),
]


def pixel_hash(path: Path) -> str:
    from PIL import Image
    with Image.open(path) as im:
        im = im.convert("RGBA")
        h = hashlib.sha256(f"{im.width}x{im.height}:".encode())
        h.update(im.tobytes())
    return h.hexdigest()


def file_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_case(case, bins):
    name, prog, args = case
    d = OUT / name
    d.mkdir(parents=True, exist_ok=True)
    paths = {"sheet": d / "sheet.png", "trace": d / "trace.json", "shot": d / "shot.png", "sram": d / "sram.sra"}
    for p in paths.values():
        if p.exists():
            p.unlink()
    argv = [str(bins[prog])] + [a.format(**{k: str(v) for k, v in paths.items()}) for a in args]
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        return name, None, f"exit {r.returncode}: {r.stderr.strip()[-2000:]}"
    out = {}
    for key, p in paths.items():
        if p.exists():
            out[f"{name}/{p.name}"] = pixel_hash(p) if p.suffix == ".png" else file_hash(p)
    return name, out, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["record", "check"])
    ap.add_argument("--bin-dir", default=os.environ.get("GOLDEN_BIN_DIR", "target/game01/release"))
    ap.add_argument("--sandbox", default="oot_sandbox")
    ap.add_argument("--viewer", default="oot_viewer")
    ap.add_argument("--only", default=None, help="only cases whose name contains this")
    ap.add_argument("--jobs", type=int, default=4)
    a = ap.parse_args()
    exe = ".exe" if sys.platform == "win32" else ""
    bins = {S: ROOT / a.bin_dir / (a.sandbox + exe), V: ROOT / a.bin_dir / (a.viewer + exe)}
    for b in bins.values():
        if not b.exists():
            sys.exit(f"missing binary {b} (build it with cargo build --release)")
    cases = [c for c in CASES if a.only is None or a.only in c[0]]
    got, errors = {}, []
    with cf.ThreadPoolExecutor(a.jobs) as ex:
        for name, out, err in ex.map(lambda c: run_case(c, bins), cases):
            if err:
                errors.append(f"{name}: {err}")
            else:
                got.update(out)
    for e in errors:
        print("ERROR", e)
    if a.mode == "record":
        if errors:
            sys.exit("not recording: some cases failed")
        HASHES.parent.mkdir(parents=True, exist_ok=True)
        lines = [f"{h}  {k}" for k, h in sorted(got.items())]
        HASHES.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        print(f"recorded {len(got)} hashes from {len(cases)} cases to {HASHES.relative_to(ROOT)}")
        return
    want = {}
    for line in HASHES.read_text(encoding="utf-8").splitlines():
        h, k = line.split(None, 1)
        if a.only is None or a.only in k.split("/")[0]:
            want[k] = h
    same = [k for k in want if got.get(k) == want[k]]
    diff = [k for k in want if k in got and got[k] != want[k]]
    missing = [k for k in want if k not in got]
    extra = [k for k in got if k not in want]
    for k in diff:
        print("DIFFERS", k)
    for k in missing:
        print("MISSING", k)
    for k in extra:
        print("NEW    ", k)
    print(f"{len(same)}/{len(want)} identical, {len(diff)} differ, {len(missing)} missing, {len(extra)} new, {len(errors)} errors")
    sys.exit(0 if not diff and not missing and not errors else 1)


if __name__ == "__main__":
    main()
