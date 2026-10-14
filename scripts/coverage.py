"""Writes docs/COVERAGE.md: what the game needs, phase by phase, against what's ported.

    target/<dir>/release/ootx census          (writes out/census.json from the pack)
    python scripts/coverage.py [--census out/census.json]

The census lists every scene layer's placed actors (rooms and transition actors) and the
overlays `oot_actors` registers. This script assigns each scene's layers to a phase of the
roadmap (PHASES, SCENES below: the plan, kept here so the page and the plan can't drift), takes
each actor's first phase as the earliest layer that places it, and reads each ported actor's
module docs for gaps it notes. Actors no scene places (spawned by other actors, items, songs)
are listed apart.

Layers: 0 and 1 are child by day and by night, 2 and 3 adult by day and by night, 4 and up the
cutscene layers. Master Quest throughout (ADR 0062): the dungeons are the debug ROM's.
"""

import argparse
import json
import os
import re
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Phase number -> (title, status). Phases 3 to 8 are the roadmap's; 9 on are the outline of
# 2026-10-13 (docs/ROADMAP.md, "The whole game").
PHASES = {
    3: ("Kokiri Forest (GAME-02)", "done"),
    4: ("The road to the Deku Tree (GAME-03)", "done"),
    6: ("The Deku Tree (GAME-05)", "done"),
    7: ("The road out of the forest (GAME-06)", "done"),
    8: ("Castle Town, Hyrule Castle and Kakariko (GAME-07)", "done"),
    9: ("The Lost Woods, the Sacred Forest Meadow and Lon Lon Ranch (GAME-08)", "in progress"),
    10: ("Death Mountain and Dodongo's Cavern", "planned"),
    11: ("Zora's River, Zora's Domain, Lake Hylia and Jabu-Jabu", "planned"),
    12: ("The Temple of Time and the adult world", "planned"),
    13: ("Epona and the adult Hyrule Field", "planned"),
    14: ("The Forest Temple", "planned"),
    15: ("The Fire Temple", "planned"),
    16: ("The Ice Cavern and the Water Temple", "planned"),
    17: ("Kakariko's shadow: the Bottom of the Well and the Shadow Temple", "planned"),
    18: ("The desert: Gerudo Valley, the fortress, the Spirit Temple", "planned"),
    19: ("Ganon's Castle and the ending", "planned"),
    20: ("Breadth: shops, games, side quests, the pause menu in full", "planned"),
}

# Scene file -> (child layers' phase, adult layers' phase, cutscene layers' phase), and
# optionally a fourth item: phases of single layers where they differ (a cutscene layer of a
# later story beat). None: not part of the game (the debug test maps) or a layer the game never
# uses for that age.
SCENES = {
    # Dungeons and their boss rooms: every layer the dungeon's phase.
    "ydan_scene": (6, 6, 6), "ydan_boss_scene": (6, 6, 6),
    "ddan_scene": (10, 10, 10), "ddan_boss_scene": (10, 10, 10),
    "bdan_scene": (11, 11, 11), "bdan_boss_scene": (11, 11, 11),
    "Bmori1_scene": (14, 14, 14), "moribossroom_scene": (14, 14, 14),
    "HIDAN_scene": (15, 15, 15), "FIRE_bs_scene": (15, 15, 15),
    "MIZUsin_scene": (16, 16, 16), "MIZUsin_bs_scene": (16, 16, 16), "ice_doukutu_scene": (16, 16, 16),
    "HAKAdan_scene": (17, 17, 17), "HAKAdan_bs_scene": (17, 17, 17), "HAKAdanCH_scene": (17, 17, 17),
    "jyasinzou_scene": (18, 18, 18), "jyasinboss_scene": (18, 18, 18), "men_scene": (18, 18, 18),
    "gerudoway_scene": (18, 18, 18),
    "ganontika_scene": (19, 19, 19), "ganon_scene": (19, 19, 19), "ganon_boss_scene": (19, 19, 19),
    "ganon_sonogo_scene": (19, 19, 19), "ganontikasonogo_scene": (19, 19, 19),
    "ganon_final_scene": (19, 19, 19), "ganon_demo_scene": (19, 19, 19), "ganon_tou_scene": (19, 19, 19),
    # Kokiri Forest and its houses. Kokiri Forest's 10 and 11 are the credits', 12 and 13 the Deku
    # Sprout's after the Forest Temple (8 and 9 hold unused cutscenes).
    "spot04_scene": (3, 12, 3, {10: 19, 11: 19, 12: 14, 13: 14}), "link_home_scene": (3, 12, 3),
    "kokiri_home_scene": (3, 12, 3), "kokiri_home3_scene": (3, 12, 3), "kokiri_home4_scene": (3, 12, 3),
    "kokiri_home5_scene": (3, 12, 3), "kokiri_shop_scene": (4, 20, 4),
    # Hyrule Field, the cutscene map, the forest's neighbours.
    # Hyrule Field: 7 the title screen's, 8 and 11 Epona's jumps, 9 Zelda after Ganon, 12
    # Impa's escort.
    "spot00_scene": (7, 13, 7, {7: 20, 8: 13, 10: 13, 11: 19, 12: 8}), "hiral_demo_scene": (7, 12, 12),
    # The meadow: 4 Sheik's minuet, 5 Saria's song.
    "spot10_scene": (9, 14, 7), "spot05_scene": (9, 14, 9, {4: 14, 5: 9}),
    # Castle Town, the castle, the Temple of Time.
    "entra_scene": (8, 12, 8), "entra_n_scene": (8, 12, 8), "enrui_scene": (12, 12, 12),
    "market_alley_scene": (8, None, 8), "market_alley_n_scene": (8, None, 8),
    "market_day_scene": (8, 12, 8), "market_night_scene": (8, 12, 8), "market_ruins_scene": (12, 12, 12),
    "shrine_scene": (8, 12, 8), "shrine_n_scene": (8, 12, 8), "shrine_r_scene": (12, 12, 12),
    # The Temple of Time: 5 and 12 to 14 the ending's. The Chamber of the Sages: 6 the sages
    # together (the ending's).
    "tokinoma_scene": (8, 12, 12, {5: 19, 12: 19, 13: 19, 14: 19}), "kenjyanoma_scene": (12, 12, 12, {6: 19}),
    "spot15_scene": (8, 19, 8), "hairal_niwa_scene": (8, None, 8), "hairal_niwa_n_scene": (8, None, 8),
    "nakaniwa_scene": (8, None, 19), "miharigoya_scene": (8, 20, 8),
    # Kakariko and the graveyard (Kakariko's 7: the mountain's eruption).
    "spot01_scene": (8, 17, 17, {7: 15}), "kakariko_scene": (8, 17, 8), "kakariko3_scene": (8, 17, 8),
    "impa_scene": (8, 20, 8), "labo_scene": (8, 20, 8), "hut_scene": (8, 17, 8),
    # GAME-07 milestone 5 took in the House of Skulltula (its family) and Kakariko's potion
    # shop (a child meets a carpenter there; the shopkeeper is the adult's).
    "kinsuta_scene": (8, 20, 8), "drag_scene": (8, 20, 8),
    # The adult graveyard and Dampé's race (the hookshot) come before the Forest Temple.
    "spot02_scene": (8, 14, 17), "hakaana_scene": (8, 17, 8), "hakaana2_scene": (8, 17, 8),
    "hakaana_ouke_scene": (8, 17, 8), "hakasitarelay_scene": (8, 14, 17),
    "yousei_izumi_tate_scene": (8, 20, 8),
    # Death Mountain: the trail's 4 the creation's, 8 the sages' (the ending's).
    "spot16_scene": (10, 15, 10, {4: 7, 8: 19}), "spot18_scene": (10, 15, 10), "spot17_scene": (15, 15, 15),
    "daiyousei_izumi_scene": (10, 15, 10),
    # Zora's waters.
    "spot03_scene": (11, 16, 11), "spot07_scene": (11, 16, 16), "spot08_scene": (11, 16, 11),
    "spot06_scene": (11, 16, 16), "hylia_labo_scene": (11, 16, 11),
    # Lon Lon Ranch: 4 Ingo's race, 5 Epona's Song (the child's), 6 on the credits'.
    "spot20_scene": (9, 13, 19, {4: 13, 5: 9}), "malon_stable_scene": (9, 13, 9), "souko_scene": (9, 13, 9),
    # The desert.
    "spot09_scene": (18, 18, 7, {6: 18}), "spot12_scene": (18, 18, 18), "spot13_scene": (18, 18, 18),
    "spot11_scene": (18, 18, 18), "tent_scene": (18, 18, 18),
    # Shops, games and side places.
    "takaraya_scene": (20, 20, 20), "shop1_scene": (20, 20, 20), "golon_scene": (20, 20, 20),
    "zoora_scene": (20, 20, 20), "alley_shop_scene": (20, 20, 20),
    "night_shop_scene": (20, 20, 20), "face_shop_scene": (20, 20, 20), "mahouya_scene": (20, 20, 20),
    "syatekijyou_scene": (20, 20, 20), "bowling_scene": (20, 20, 20), "turibori_scene": (20, 20, 20),
    "yousei_izumi_yoko_scene": (20, 20, 20),
    # The grottos: one scene for the holes all over the world, optional (heart pieces,
    # Skulltulas, fairies, scrubs).
    "kakusiana_scene": (20, 20, 20),
    # The debug ROM's test maps: not the game.
    "test01_scene": None, "besitu_scene": None, "depth_test_scene": None, "syotes_scene": None,
    "syotes2_scene": None, "sutaru_scene": None, "hairal_niwa2_scene": None, "sasatest_scene": None,
    "testroom_scene": None,
}

# Actors the story needs later than their first placement: placed in an early layer but
# killed there, or optional until later.
ACTORS = {
    "Item_Ocarina": 11,  # the Ocarina of Time in the moat, after the third stone
    "En_Mm": 20,  # the running man (the mask trade)
    "En_Cow": 9,  # Link's house's (Malon's prize) and the ranch's
    "Obj_Dekujr": 14,  # the Deku Sprout: killed for a child outside a cutscene layer, and without
    # the Forest Medallion (ObjDekujr_Init)
    "En_Po_Relay": 14,  # Dampé's race: in Dampé's grave (hakasitarelay room 0), which a child
    # can't open (Bg_Spot02_Objects type 1 seals it for him); the child's layers place it anyway
}

# What a ported actor's module docs say when it leaves something out.
GAP = re.compile(r"not ported|isn't ported|aren't ported|placeholder|left out|logged|isn't drawn|aren't drawn", re.I)


def layer_phase(scene, layer):
    p = SCENES.get(scene)
    if p is None:
        return None
    child, adult, cs = p[:3]
    if len(p) > 3 and layer in p[3]:
        return p[3][layer]
    if layer <= 1:
        return child
    if layer <= 3:
        return adult
    return cs


def module_doc(name):
    path = os.path.join(ROOT, "crates", "game", "oot_actors", "src", name.lower() + ".rs")
    if not os.path.exists(path):
        return None
    doc = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            if not line.startswith("//!"):
                break
            doc.append(line[3:].strip())
    return " ".join(doc)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--census", default=os.path.join(ROOT, "out", "census.json"))
    ap.add_argument("--out", default=os.path.join(ROOT, "docs", "COVERAGE.md"))
    args = ap.parse_args()
    with open(args.census, encoding="utf-8") as f:
        c = json.load(f)

    unknown = [s["file"] for s in c["scenes"] if s["file"] not in SCENES]
    if unknown:
        raise SystemExit(f"scenes with no phase in SCENES: {unknown}")

    ported = set(c["ported"])
    overlays = [o["name"] for o in c["overlays"]]
    first = {}  # actor -> phase
    where = defaultdict(set)  # actor -> scenes
    scene_rows = []
    for s in c["scenes"]:
        phases = set()
        names = set()
        for l in s["layers"]:
            ph = layer_phase(s["file"], l["layer"])
            if ph is None:
                continue
            phases.add(ph)
            for name in l["actors"]:
                names.add(name)
                where[name].add(s["file"])
                if name not in first or ph < first[name]:
                    first[name] = ph
        if SCENES[s["file"]] is not None:
            scene_rows.append((s, sorted(phases), names))

    for name, ph in ACTORS.items():
        if name in first:
            first[name] = ph

    gaps = {}
    for name in ported:
        doc = module_doc(name)
        if doc and GAP.search(doc):
            gaps[name] = True

    def status(name):
        if name in ported:
            return "ported, gaps noted" if name in gaps else "ported"
        return "not yet"

    out = []
    w = out.append
    w("# Coverage: the game, phase by phase")
    w("")
    w("Generated by `scripts/coverage.py` from `ootx census` (the pack's scenes and `oot_actors`'")
    w("registrations); don't edit it by hand. To refresh it: `scripts\\run\\coverage.bat`. The phases")
    w("are [ROADMAP.md](ROADMAP.md)'s; each scene's layers are assigned in the script's `SCENES` table")
    w("(child layers 0 and 1, adult 2 and 3, cutscene layers 4 and up). An actor counts toward the")
    w("first phase whose scenes place it. \"Gaps noted\": its module docs say something is left out")
    w("(a placeholder spawned, a part not ported, a call logged); BACKLOG.md has the details.")
    w("")
    placed = set(first)
    placed_ported = placed & ported
    spawned = sorted(set(overlays) - placed)
    w(f"**Actor overlays:** {len(overlays)} in the table, {len(ported)} ported "
      f"({100 * len(ported) // len(overlays)}%). Placed in the game's scenes: {len(placed)}, "
      f"{len(placed_ported)} of them ported. Only spawned (by actors, items, songs, cutscenes): "
      f"{len(spawned)}, {len(set(spawned) & ported)} ported.")
    w("")
    w("## By phase")
    w("")
    w("| Phase | | Actors first placed here | Ported | Still to port |")
    w("|---|---|---|---|---|")
    for ph, (title, st) in PHASES.items():
        names = sorted(n for n, p in first.items() if p == ph)
        if not names and st != "done":
            continue
        todo = [n for n in names if n not in ported]
        w(f"| {ph}: {title} | {st} | {len(names)} | {len(names) - len(todo)} | "
          f"{', '.join(f'`{n}`' for n in todo) if todo else '-'} |")
    w("")
    w("## Ported with gaps noted")
    w("")
    w(", ".join(f"`{n}`" for n in sorted(gaps)) or "-")
    w("")
    w("## Only spawned")
    w("")
    w("No scene places these; something spawns them (an actor, an item, a song, a cutscene), so")
    w("each comes with its spawner.")
    w("")
    w("| Overlay | Status |")
    w("|---|---|")
    for n in spawned:
        w(f"| `{n}` | {status(n)} |")
    w("")
    w("## Scenes")
    w("")
    w("| Scene | Phases | Draw config | Actors placed | Ported |")
    w("|---|---|---|---|---|")
    for s, phases, names in sorted(scene_rows, key=lambda r: (min(r[1]) if r[1] else 99, r[0]["id"])):
        dc = s["draw_config"] + (" (ported)" if s["draw_config_ported"] else "")
        w(f"| `{s['file'][:-6]}` ({s['enum'][6:]}) | {', '.join(map(str, phases))} | `{dc}` | "
          f"{len(names)} | {len(names & ported)} |")
    w("")
    with open(args.out, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(out))
    print(f"{args.out}: {len(overlays)} overlays, {len(ported)} ported, {len(placed)} placed, {len(spawned)} only spawned")


if __name__ == "__main__":
    main()
