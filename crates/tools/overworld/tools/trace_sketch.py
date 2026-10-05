"""Trace a hand-drawn sketch into an overworld level document.

    python overworld/tools/trace_sketch.py sketch.png out.json [--scale 3.5] [--crop x0,y0,x1,y1]
        [--region blue:z=120] [--region red:kind=water,z=-100,surface=-20] ...

The dark closed stroke is the outline. Each closed stroke in a colour (blue, red, green, yellow,
orange, purple) is a region; `--region colour:key=value,...` sets its fields (default z 120).
Strokes are traced along their middle and reduced to nodes (Douglas-Peucker, `--tol` pixels for
the outline, `--region-tol` for regions, which are usually drawn as clicked polygons). The
image centre is the origin; `--scale` is units per pixel (3.5 makes a 970-wide sketch about
Kokiri village's size). Image y runs down, so y is flipped (north is up in the sketch).
"""

from __future__ import annotations

import argparse
import json

import cv2
import numpy as np

HUES = {"red": (0, 10), "orange": (10, 22), "yellow": (22, 35), "green": (35, 85), "blue": (95, 130),
        "purple": (130, 165)}


def loops_of(mask, tol, min_area=400):
    """Closed strokes in a mask -> their centre lines as polygons (pixel coords), biggest first."""
    h, w = mask.shape
    # what the outside can reach without crossing a stroke; the rest is stroke or inside
    filled = np.zeros((h + 2, w + 2), np.uint8)
    reach = mask.copy().astype(np.uint8) * 255
    cv2.floodFill(reach, filled, (0, 0), 128)
    inside = (reach != 128).astype(np.uint8)
    # stroke centre: shrink by half the stroke width
    sw = stroke_width(mask)
    k = max(1, int(round(sw / 2)))
    inside = cv2.erode(inside, np.ones((2 * k + 1, 2 * k + 1), np.uint8)) if k > 1 else inside
    contours, _ = cv2.findContours(inside, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    out = []
    for c in sorted(contours, key=cv2.contourArea, reverse=True):
        if cv2.contourArea(c) < min_area:
            continue
        poly = cv2.approxPolyDP(c, tol, True)[:, 0, :].astype(float)
        out.append(poly)
    return out


def stroke_width(mask):
    d = cv2.distanceTransform(mask.astype(np.uint8), cv2.DIST_L2, 3)
    vals = d[d > 0]
    return 2 * float(np.percentile(vals, 90)) if len(vals) else 3.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("out")
    ap.add_argument("--scale", type=float, default=3.5)
    ap.add_argument("--crop", default=None)
    ap.add_argument("--tol", type=float, default=3.0)
    ap.add_argument("--region-tol", type=float, default=4.0)
    ap.add_argument("--region", action="append", default=[])
    ap.add_argument("--name", default=None)
    a = ap.parse_args()

    im = cv2.imread(a.image)
    if a.crop:
        x0, y0, x1, y1 = map(int, a.crop.split(","))
        im = im[y0:y1, x0:x1]
    h, w = im.shape[:2]
    hsv = cv2.cvtColor(im, cv2.COLOR_BGR2HSV)
    dark = (im.max(axis=2) < 90)
    coloured = (hsv[..., 1] > 90) & (hsv[..., 2] > 60)

    cx, cy = w / 2, h / 2
    to_world = lambda p: [round((p[0] - cx) * a.scale, 1), round(-(p[1] - cy) * a.scale, 1)]

    outline = loops_of(dark, a.tol)
    if not outline:
        raise SystemExit("no closed dark stroke found")
    doc = {"name": a.name or a.out.replace("\\", "/").split("/")[-1].rsplit(".", 1)[0],
           "outline": {"nodes": [to_world(p) for p in outline[0]], "z": 0},
           "regions": []}

    opts = {}
    for r in a.region:
        colour, _, kv = r.partition(":")
        d = {}
        for item in filter(None, kv.split(",")):
            k, _, v = item.partition("=")
            d[k] = v if k in ("kind", "name", "edge") else float(v)
        opts[colour] = d
    for colour, (lo, hi) in HUES.items():
        m = coloured & (hsv[..., 0] >= lo) & (hsv[..., 0] < hi)
        if colour == "red":
            m |= coloured & (hsv[..., 0] >= 170)
        if m.sum() < 50:
            continue
        for k, poly in enumerate(loops_of(m, a.region_tol, min_area=100)):
            reg = {"name": colour if k == 0 else f"{colour}_{k}", "nodes": [to_world(p) for p in poly], "z": 120}
            reg.update(opts.get(colour, {}))
            doc["regions"].append(reg)

    with open(a.out, "w") as f:
        json.dump(doc, f, indent=1)
    print(f"{a.out}: outline {len(doc['outline']['nodes'])} nodes, "
          + ", ".join(f"{r['name']} {len(r['nodes'])} nodes" for r in doc["regions"])
          + f"; {w * a.scale:.0f} x {h * a.scale:.0f} units")


if __name__ == "__main__":
    main()
