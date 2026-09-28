"""Render regression for the spike behaviour: runs every headless case, hashes the output,
and records or checks the hashes.

    python scripts/golden.py record [--bin-dir target/<you>/release]
    python scripts/golden.py check  [--bin-dir target/<you>/release] [--only substr]

Images and traces are written to the git-ignored out/golden/ and stay local. Only the hashes
(golden/renders.sha256) are committed. PNGs are hashed on their decoded RGBA pixels and size,
so encoder settings don't matter; JSON traces are hashed as bytes.

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
    # Spike 02 viewer.
    ("viewer_link_sheet", V, ["--sheet", "{shot}"]),
    ("viewer_link_sheet_child", V, ["--age", "child", "--sheet", "{shot}"]),
    ("viewer_link_sword", V, ["--sheet", "{shot}", "--group", "SWORD", "--shield", "HYLIAN",
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
    paths = {"sheet": d / "sheet.png", "trace": d / "trace.json", "shot": d / "shot.png"}
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
