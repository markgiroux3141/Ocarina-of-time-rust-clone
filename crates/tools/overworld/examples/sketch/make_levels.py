"""The test levels from the user's sketch (outline_blue.png, traced into traced.json):

- sketch_plateau: the blue region raised 120, and the north-east lobe raised 160 against the
  wall, so the rim rises over it.
- sketch_pond: the blue region as a pond (bed -100, surface -20), the same north-east plateau.
- sketch_paths: the island and the north-east plateau at 360, with ramps up to both, a rock
  bridge between them high enough to walk under, and a path that climbs as an embankment then
  carries on as a bridge.
- sketch_bumpy: sketch_paths with bumps on the ground and both plateaus.
- sketch_hills: sketch_bumpy with painted terrain (the grid the editor's brush paints): a broad hill
  under the island, which lifts it with its ramps, a rise in the open south-west and a hollow in the
  middle of the ground.

Hand-placed points are in sketch pixels, so the level's scale is the trace's alone:

    python overworld/tools/trace_sketch.py overworld/examples/sketch/outline_blue.png \\
        overworld/examples/sketch/traced.json --scale 8 --tol 5 --region blue:z=120 --name sketch
    python overworld/examples/sketch/make_levels.py
    overworld/target/release/overworld build overworld/examples/sketch/sketch_paths.json <out> --textures ...
"""

import copy
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PX = 8.0           # units per sketch pixel (trace_sketch's --scale); 3.5 made Kokiri-village size
W, H = 969, 668    # the sketch's size in pixels


def world(px, py):
    return [round((px - W / 2) * PX, 1), round(-(py - H / 2) * PX, 1)]


def node(px, py, z=None):
    return world(px, py) + ([z] if z is not None else [])


def paint_hills(hills, cell=50.0, chunk=16):
    """A terrain grid (overworld/src/terrain.rs) holding smooth domes: offsets at grid nodes, in
    chunks of chunk x chunk nodes keyed "cx,cy", row by row."""
    import math
    nodes = {}
    for (x, y, r, h) in hills:
        for i in range(math.floor((x - r) / cell), math.ceil((x + r) / cell) + 1):
            for j in range(math.floor((y - r) / cell), math.ceil((y + r) / cell) + 1):
                d = math.hypot(i * cell - x, j * cell - y) / r
                if d < 1:
                    t = max(0.0, (d - 0.25) / 0.75)
                    nodes[(i, j)] = nodes.get((i, j), 0.0) + h * (1 - t * t * (3 - 2 * t))
    chunks = {}
    for (i, j), v in nodes.items():
        c = chunks.setdefault(f"{i // chunk},{j // chunk}", [0.0] * chunk * chunk)
        c[(j % chunk) * chunk + (i % chunk)] = round(v, 2)
    return {"cell": cell, "detail": 100.0, "chunks": dict(sorted(chunks.items()))}


def main():
    base = json.load(open(os.path.join(HERE, "traced.json")))
    out = base["outline"]["nodes"]
    blue = base["regions"][0]
    # the north-east lobe: outline nodes 4, 3, 2, 1, 0, 79, 78 along the wall, closed by a curve
    # through the lobe's middle
    wall = [out[i] for i in (4, 3, 2, 1, 0, 79, 78)]
    ne = {"name": "north_east", "nodes": wall + [world(770, 170), world(690, 140)], "z": 160}

    plateau = copy.deepcopy(base)
    plateau["name"] = "sketch_plateau"
    plateau["regions"] = [dict(blue, name="blue_plateau", z=120), ne]

    pond = copy.deepcopy(base)
    pond["name"] = "sketch_pond"
    pond["regions"] = [dict(blue, name="blue_pond", kind="water", z=-100, surface=-20), ne]

    # paths: ends without a height take the floor's
    paths = copy.deepcopy(base)
    paths["name"] = "sketch_paths"
    paths["regions"] = [dict(blue, name="island", z=360), dict(ne, z=360)]
    paths["paths"] = [
        # along the east wall (its side overlaps the wall, which cuts it) up onto the plateau
        {"name": "east_ramp", "nodes": [node(773, 377), node(776, 117)], "width": 220},
        # from the ground south-east of the island up onto it
        {"name": "island_ramp", "nodes": [node(613, 405), node(442, 263)], "width": 220},
        # island to plateau, a rock arch over open ground
        {"name": "bridge", "nodes": [node(442, 205), node(685, 105)], "mode": "floating", "width": 220},
        # climbs as an embankment, then carries on as a bridge onto the island's west side
        {"name": "climb", "nodes": [node(356, 420), node(350, 320, 225), node(399, 234)],
         "modes": ["attached", "floating"], "width": 220},
    ]

    bumpy = copy.deepcopy(paths)
    bumpy["name"] = "sketch_bumpy"
    bumpy["outline"]["noise"] = {"amplitude": 70, "scale": 900, "edge": 300}
    for r in bumpy["regions"]:
        r["noise"] = {"amplitude": 30, "scale": 500, "edge": 180}

    hills = copy.deepcopy(bumpy)
    hills["name"] = "sketch_hills"
    hills["terrain"] = paint_hills([
        # (x, y, radius, height): a raise brush's shape, a smooth dome with a flat-ish top
        (-550, 700, 1500, 160),
        (-2200, -1300, 1100, 120),
        (300, -700, 700, -70),
    ])

    for d in (plateau, pond, paths, bumpy, hills):
        path = os.path.join(HERE, d["name"] + ".json")
        with open(path, "w") as f:
            json.dump(d, f, indent=1)
        print(path)


if __name__ == "__main__":
    main()
