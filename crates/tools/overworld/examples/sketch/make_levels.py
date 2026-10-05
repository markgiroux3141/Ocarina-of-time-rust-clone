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
- sketch_village: sketch_pond at medium detail with Kokiri's houses, stumps, a hedge and stepping
  stones across the pond (kit pieces, `props`), a ramp up to the north-east plateau where the
  Know-It-All Brothers live, a lookout joined to it by a hanging bridge, the south-west rise, which
  Link's house stands on, dirt paths between the doors and fences (`lines`).

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

    village = copy.deepcopy(pond)
    village["name"] = "sketch_village"
    village["settings"] = {"detail": "medium"}
    # a lookout west of the plateau, at its height (sharp corners: flat walls, the north and south
    # ones parallel for the crawlspace), with a ramp up and a hanging bridge across
    village["regions"] = village["regions"] + [
        {"name": "lookout", "nodes": [[350, 1060, 1], [920, 1060, 1], [920, 1480, 1], [350, 1480, 1]], "z": 160}]
    village["paths"] = [{"name": "plateau_ramp", "nodes": [[1900, 700], [2100, 1600]], "width": 220},
                        {"name": "lookout_ramp", "nodes": [[450, 500], [600, 1250]], "width": 180}]
    village["terrain"] = paint_hills([(-2200, -1300, 1100, 120)])

    def prop(piece, x, y, yaw=0, scale=None):
        p = {"piece": piece, "at": [x, y], "yaw": yaw}
        if scale:
            p["scale"] = scale
        return p

    village["props"] = [
        prop("link_house", -1500, -1500),
        prop("mido_house", -2000, 1200, -90),
        prop("saria_house", 200, -1300, 20),
        prop("twins_house", 1200, -300, 90),
        prop("shop", 2300, -300, 120),
        prop("knowitall_house", 2300, 1900, 180),
        prop("stump_post", -200, -400),
        prop("stump_post_tall", 100, -300),
        prop("hedge", -1700, 600, 30, [1.5, 1.5, 1]),
        prop("stone_small", -700, 600),
        prop("stone_medium", -560, 730),
        prop("stone_large", -420, 860),
        # set into walls: a log exit in the edge of the world, a crawlspace through the lookout,
        # vines up its east face
        prop("log_tunnel", -2900, 1350),
        prop("crawlspace", 800, 1040),
        prop("vines", 940, 1200),
    ]
    # dirt paths between the doors, as Kokiri's
    village["lines"] = [
        {"name": "main path", "kind": "dirt", "nodes": [[-1470, -1180], [-900, -800], [-250, -650], [150, -1050]]},
        {"name": "east path", "kind": "dirt", "nodes": [[-250, -650], [500, -350], [1100, -330], [1700, -250], [2150, -380]]},
        {"name": "plateau path", "kind": "dirt", "nodes": [[1700, -250], [1900, 500], [2100, 1500], [2280, 1780]]},
        # a pen round the stumps, open on the path side, and the lattice by the hedge
        {"name": "pen", "kind": "fence", "nodes": [[-40, -230], [-420, -260], [-440, -520], [-330, -560]]},
        {"name": "lattice", "kind": "lattice", "nodes": [[-1450, 350], [-1250, 520]]},
        {"name": "rope bridge", "kind": "bridge", "nodes": [[880, 1400], [1780, 1760]]},
    ]

    for d in (plateau, pond, paths, bumpy, hills, village):
        path = os.path.join(HERE, d["name"] + ".json")
        with open(path, "w") as f:
            json.dump(d, f, indent=1)
        print(path)


if __name__ == "__main__":
    main()
