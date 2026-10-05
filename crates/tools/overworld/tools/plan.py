"""A plan of a built level (level.json from `overworld build`): floors coloured by height, the
bank in grey shaded by height (so the rim's rise shows), water blue, walls dark, the tree line green.

    python overworld/tools/plan.py <out dir>/level.json <plan.png> [--doc level_doc.json]

With --doc, the document's nodes are drawn too (outline red, regions in blue).
"""

import argparse
import json

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.collections import PolyCollection  # noqa: E402


def tris(obj):
    v, t = obj["verts"], obj["tris"]
    P = [(v[3 * i], v[3 * i + 1], v[3 * i + 2]) for i in range(len(v) // 3)]
    return [[P[t[k]], P[t[k + 1]], P[t[k + 2]]] for k in range(0, len(t), 3)], obj


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("level")
    ap.add_argument("png")
    ap.add_argument("--doc")
    a = ap.parse_args()
    lvl = json.load(open(a.level))
    objs = {o["name"]: o for o in lvl["objects"]}
    fig, ax = plt.subplots(figsize=(13, 9.5))
    ax.set_aspect("equal")
    ax.set_facecolor("#202020")

    def poly(name, face_color, cmap=None, vmin=None, vmax=None, alpha=1.0):
        if name not in objs:
            return None
        T, _ = tris(objs[name])
        polys = [[(p[0], p[1]) for p in t] for t in T]
        z = [sum(p[2] for p in t) / 3 for t in T]
        if cmap:
            pc = PolyCollection(polys, array=z, cmap=cmap, edgecolors="none", alpha=alpha)
            pc.set_clim(vmin, vmax)
        else:
            pc = PolyCollection(polys, facecolors=face_color, edgecolors="none", alpha=alpha)
        ax.add_collection(pc)
        return pc

    ground = objs["ground"]
    zs = ground["verts"][2::3]
    pc = poly("ground", None, "viridis", min(zs), max(max(zs), 1))
    poly("bank", None, "Greys", lvl["rim"][0] - 400, lvl["rim"][1] + 100)
    poly("water", "#3a7bd5", alpha=0.75)
    poly("bridges", "#ff9800", alpha=0.55)
    for name, colour, lw in (("walls", "#111111", 2.2), ("trees", "#2e7d32", 3.0)):
        if name in objs:
            T, _ = tris(objs[name])
            for t in T:
                ax.plot([p[0] for p in t], [p[1] for p in t], color=colour, lw=lw, solid_capstyle="round")
    if a.doc:
        d = json.load(open(a.doc))
        ns = d["outline"]["nodes"]
        ax.plot([n[0] for n in ns], [n[1] for n in ns], "o", color="#e53935", ms=3)
        for r in d.get("regions", []):
            ax.plot([n[0] for n in r["nodes"]], [n[1] for n in r["nodes"]], "o", color="#64b5f6", ms=4)
    ax.autoscale_view()
    cb = fig.colorbar(pc, ax=ax, shrink=0.6)
    cb.set_label("floor height")
    ax.set_title(f"{lvl.get('name', '')}: rim {lvl['rim'][0]:.0f} to {lvl['rim'][1]:.0f} (bank shaded by height)")
    fig.savefig(a.png, dpi=90, bbox_inches="tight")
    print(a.png)


if __name__ == "__main__":
    main()
