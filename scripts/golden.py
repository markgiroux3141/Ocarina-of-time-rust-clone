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
    # GAME-05 milestone 6a: Queen Gohma's room from its entrance's spawn: in, her intro, looked at
    # with the slingshot, the fight (a seed into her red eye, jump slashes, a seed on the ceiling),
    # her death and the heart container taken: the trace and the end; then the run cut at frame
    # 560 (her drop: the boss's title card) and 1430 (her death: her textures half erased, her
    # pieces, the room flashing blue).
    ("gohma", S, ["--entrance", "ENTR_DEKU_TREE_BOSS_0", "--child", "--preset", "deku-tree-gohma", "--script", "gohma", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("gohma_title", S, ["--entrance", "ENTR_DEKU_TREE_BOSS_0", "--child", "--preset", "deku-tree-gohma", "--script", "gohma", "--frames", "560", "--screenshot", "{shot}"]),
    ("gohma_decay", S, ["--entrance", "ENTR_DEKU_TREE_BOSS_0", "--child", "--preset", "deku-tree-gohma", "--script", "gohma", "--frames", "1430", "--screenshot", "{shot}"]),
    # GAME-05 milestone 6b: Queen Gohma's cleared room, before the blue warp she leaves: into it,
    # the float, the white fade out with the Kokiri Emerald, Kokiri Forest by blue warp and the
    # Deku Tree's emerald cutscene, part 1, to its terminator: the trace and the end; then the run
    # cut at frame 200 (Link in the warp's rays) and 700 (the Deku Tree's text).
    ("blue_warp", S, ["--entrance", "ENTR_DEKU_TREE_BOSS_0", "--child", "--preset", "deku-tree-gohma-cleared", "--script", "blue-warp", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("blue_warp_float", S, ["--entrance", "ENTR_DEKU_TREE_BOSS_0", "--child", "--preset", "deku-tree-gohma-cleared", "--script", "blue-warp", "--frames", "200", "--screenshot", "{shot}"]),
    ("blue_warp_forest", S, ["--entrance", "ENTR_DEKU_TREE_BOSS_0", "--child", "--preset", "deku-tree-gohma-cleared", "--script", "blue-warp", "--frames", "700", "--screenshot", "{shot}"]),
    # GAME-05 milestone 6c: room 9's debug start, the room cleared (its door to room 11 open):
    # through the door, onto room 11's floor (exit 2) and into Queen Gohma's room, standing in its
    # corridor: the trace and the end; then the run cut at frame 265 (Link through the door into
    # room 11).
    ("boss_room", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-slingshot", "--script", "boss-room", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("boss_room_door", S, ["--entrance", "ENTR_DEKU_TREE_0", "--child", "--preset", "deku-tree-slingshot", "--script", "boss-room", "--frames", "265", "--screenshot", "{shot}"]),
    # GAME-06 milestone 1a: Demo_Effect, Kokiri Forest's cutscene layers 6 (the emerald) and 4 (Farore).
    ("emerald", S, ["--entrance", "ENTR_KOKIRI_FOREST_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF2", "--script", "emerald", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("emerald_light", S, ["--entrance", "ENTR_KOKIRI_FOREST_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF2", "--script", "emerald", "--frames", "885", "--screenshot", "{shot}"]),
    ("emerald_floats", S, ["--entrance", "ENTR_KOKIRI_FOREST_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF2", "--script", "emerald", "--frames", "1100", "--screenshot", "{shot}"]),
    ("farore", S, ["--entrance", "ENTR_KOKIRI_FOREST_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF0", "--script", "idle", "--frames", "30", "--screenshot", "{shot}"]),
    # GAME-06 milestone 1b: the creation, the chain's parts 2 to 9 from the cutscene map's layer 5
    # to Kokiri Forest's entrance 11: the trace and the end; then three parts from their debug
    # starts: the goddesses' blue rain (the cutscene map's layer 4, frame 210), Din's rocks
    # (Gerudo Valley's layer 5, frame 75), Nayru's light rings (Death Mountain Trail's layer 4,
    # frame 107).
    ("creation", S, ["--entrance", "ENTR_CUTSCENE_MAP_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF1", "--script", "creation", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("creation_rain", S, ["--entrance", "ENTR_CUTSCENE_MAP_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF0", "--script", "idle", "--frames", "210", "--screenshot", "{shot}"]),
    ("creation_rocks", S, ["--entrance", "ENTR_GERUDO_VALLEY_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF1", "--script", "idle", "--frames", "75", "--screenshot", "{shot}"]),
    ("creation_nayru", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_0", "--child", "--preset", "deku-tree-dead", "--cutscene", "0xFFF0", "--script", "idle", "--frames", "107", "--screenshot", "{shot}"]),
    # GAME-06 milestone 2: out of the forest, from ENTR_KOKIRI_FOREST_11 (where the chain leaves
    # Link): Mido's talk, the Lost Woods' bridge (Saria, the Fairy Ocarina), Hyrule Field's intro,
    # the owl's talk and flight: the trace and the end; then the run cut at frame 1800 (Saria and
    # Link on the bridge), 2000 (the ocarina held out) and 3800 (the owl talking).
    ("farewell", S, ["--entrance", "ENTR_KOKIRI_FOREST_11", "--child", "--preset", "deku-tree-dead", "--script", "farewell", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("farewell_bridge", S, ["--entrance", "ENTR_KOKIRI_FOREST_11", "--child", "--preset", "deku-tree-dead", "--script", "farewell", "--frames", "1800", "--screenshot", "{shot}"]),
    ("farewell_ocarina", S, ["--entrance", "ENTR_KOKIRI_FOREST_11", "--child", "--preset", "deku-tree-dead", "--script", "farewell", "--frames", "2000", "--screenshot", "{shot}"]),
    ("farewell_owl", S, ["--entrance", "ENTR_KOKIRI_FOREST_11", "--child", "--preset", "deku-tree-dead", "--script", "farewell", "--frames", "3800", "--screenshot", "{shot}"]),
    # GAME-06 milestone 3: the clock, Hyrule Field from 17:00 to the night.
    ("dusk", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "deku-tree-dead", "--time", "17:00", "--script", "dusk", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("dusk_sunset", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "deku-tree-dead", "--time", "17:00", "--script", "dusk", "--frames", "150", "--screenshot", "{shot}"]),
    ("dusk_bridge", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "deku-tree-dead", "--time", "17:00", "--script", "dusk", "--frames", "350", "--screenshot", "{shot}"]),
    # GAME-06 milestone 4: Hyrule Field by day (a Peahat fought, Castle Town's entrance) and at
    # 20:00 (Stalchildren fought, the owl at Kakariko's stairs, the village).
    ("field", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "deku-tree-dead", "--script", "field", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("field_peahat", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "deku-tree-dead", "--script", "field", "--frames", "330", "--screenshot", "{shot}"]),
    ("field_night", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "deku-tree-dead", "--time", "20:00", "--script", "field-night", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("field_night_fight", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "deku-tree-dead", "--time", "20:00", "--script", "field-night", "--frames", "80", "--screenshot", "{shot}"]),
    # GAME-07 milestone 1: Castle Town by day, from its entrance into the market, a townsman's
    # and Malon's talks, out to the castle's road and its intro (gHyruleCastleIntroCs): the trace
    # and the end; then the run cut at frame 380 (the townsman's text), 860 (Malon's) and 1120
    # (the castle's intro).
    ("market", S, ["--entrance", "ENTR_MARKET_ENTRANCE_DAY_0", "--child", "--preset", "deku-tree-dead", "--script", "market", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("market_townsman", S, ["--entrance", "ENTR_MARKET_ENTRANCE_DAY_0", "--child", "--preset", "deku-tree-dead", "--script", "market", "--frames", "380", "--screenshot", "{shot}"]),
    ("market_malon", S, ["--entrance", "ENTR_MARKET_ENTRANCE_DAY_0", "--child", "--preset", "deku-tree-dead", "--script", "market", "--frames", "860", "--screenshot", "{shot}"]),
    ("market_castle", S, ["--entrance", "ENTR_MARKET_ENTRANCE_DAY_0", "--child", "--preset", "deku-tree-dead", "--script", "market", "--frames", "1120", "--screenshot", "{shot}"]),
    # GAME-07 milestone 2: Hyrule Castle's grounds with the Cucco (castle-cucco): Talon shown it and
    # run home (by night now), two crates pushed into the moat, up them and through the drain into
    # the guards' courtyard (its circle, CIRCLE(WAVE, BLACK, FAST), opening).
    ("castle", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-cucco", "--script", "castle", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("castle_talon", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-cucco", "--script", "castle", "--frames", "3260", "--screenshot", "{shot}"]),
    ("castle_crate", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-cucco", "--script", "castle", "--frames", "4371", "--screenshot", "{shot}"]),
    ("castle_drain", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-cucco", "--script", "castle", "--frames", "4990", "--screenshot", "{shot}"]),
    ("castle_wave", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-cucco", "--script", "castle", "--frames", "5127", "--screenshot", "{shot}"]),
    # The same, caught (castle-egg): "Hey you!" (0x702D), the starburst circle shutting and the
    # throw-out by the gate (ENTR_HYRULE_CASTLE_4) opening.
    ("castle_caught", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-egg", "--script", "castle-caught", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("castle_caught_shut", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-egg", "--script", "castle-caught", "--frames", "1434", "--screenshot", "{shot}"]),
    ("castle_caught_open", S, ["--entrance", "ENTR_HYRULE_CASTLE_0", "--child", "--preset", "castle-egg", "--script", "castle-caught", "--frames", "1460", "--screenshot", "{shot}"]),
    # GAME-07 milestone 3: the ocarina in Hyrule Field at 18:30 (ocarina-songs): Saria's Song (Navi
    # calls, Saria talks), the Sun's Song (the clock run to dawn), and the Song of Storms at the
    # fairy spot (Shot_Sun's fairy); then the run cut at frame 95 (the staff's notes), 140 (Saria's
    # Song's leaves, "You played"), 560 (the Sun's Song's light) and 880 (the storm's rain).
    ("ocarina", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "ocarina-songs", "--time", "18:30", "--script", "ocarina", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("ocarina_staff", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "ocarina-songs", "--time", "18:30", "--script", "ocarina", "--frames", "95", "--screenshot", "{shot}"]),
    ("ocarina_saria", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "ocarina-songs", "--time", "18:30", "--script", "ocarina", "--frames", "140", "--screenshot", "{shot}"]),
    ("ocarina_sun", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "ocarina-songs", "--time", "18:30", "--script", "ocarina", "--frames", "560", "--screenshot", "{shot}"]),
    ("ocarina_storm", S, ["--entrance", "ENTR_HYRULE_FIELD_0", "--child", "--preset", "ocarina-songs", "--time", "18:30", "--script", "ocarina", "--frames", "880", "--screenshot", "{shot}"]),
    # GAME-07 milestone 4: from the castle's drain into the guards' courtyard (castle-courtyard), past
    # the patrols unseen, Zelda's talk through the legend and the window to her letter, Impa's
    # lullaby (a wrong note, then right) and her goodbye in Hyrule Field (ENTR_HYRULE_FIELD_17); then
    # the run cut at frame 900 (the patrols), 7100 (Zelda's talk), 10600 (Ganondorf at the window),
    # 12200 (the letter), 13300 (Impa's lesson) and 14200 (Impa in the field).
    ("zelda", S, ["--entrance", "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", "--child", "--preset", "castle-courtyard", "--script", "zelda", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("zelda_patrols", S, ["--entrance", "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", "--child", "--preset", "castle-courtyard", "--script", "zelda", "--frames", "900", "--screenshot", "{shot}"]),
    ("zelda_talk", S, ["--entrance", "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", "--child", "--preset", "castle-courtyard", "--script", "zelda", "--frames", "7100", "--screenshot", "{shot}"]),
    ("zelda_ganondorf", S, ["--entrance", "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", "--child", "--preset", "castle-courtyard", "--script", "zelda", "--frames", "10600", "--screenshot", "{shot}"]),
    ("zelda_letter", S, ["--entrance", "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", "--child", "--preset", "castle-courtyard", "--script", "zelda", "--frames", "12200", "--screenshot", "{shot}"]),
    ("zelda_lullaby", S, ["--entrance", "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", "--child", "--preset", "castle-courtyard", "--script", "zelda", "--frames", "13300", "--screenshot", "{shot}"]),
    ("zelda_impa", S, ["--entrance", "ENTR_CASTLE_COURTYARD_GUARDS_DAY_0", "--child", "--preset", "castle-courtyard", "--script", "zelda", "--frames", "14200", "--screenshot", "{shot}"]),
    # GAME-07 milestone 5: Kakariko (Route::Kakariko): its intro, three cuccos carried to Anju's pen
    # (the third glided off the low roof), the letter shown to the gate guard and the gate opened.
    ("kakariko", S, ["--entrance", "ENTR_KAKARIKO_VILLAGE_0", "--child", "--preset", "kakariko-letter", "--script", "kakariko", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("kakariko_carry", S, ["--entrance", "ENTR_KAKARIKO_VILLAGE_0", "--child", "--preset", "kakariko-letter", "--script", "kakariko", "--frames", "600", "--screenshot", "{shot}"]),
    ("kakariko_throw", S, ["--entrance", "ENTR_KAKARIKO_VILLAGE_0", "--child", "--preset", "kakariko-letter", "--script", "kakariko", "--frames", "908", "--screenshot", "{shot}"]),
    ("kakariko_glide", S, ["--entrance", "ENTR_KAKARIKO_VILLAGE_0", "--child", "--preset", "kakariko-letter", "--script", "kakariko", "--frames", "2140", "--screenshot", "{shot}"]),
    ("kakariko_gate", S, ["--entrance", "ENTR_KAKARIKO_VILLAGE_0", "--child", "--preset", "kakariko-letter", "--script", "kakariko", "--frames", "4000", "--screenshot", "{shot}"]),
    # GAME-07 milestone 6: the graveyard (Route::Graveyard) at 18:30: its intro, a dig with Dampé, the
    # lullaby at the royal tomb (its gravestone blown up), the tomb's Keese and door, past the ReDeads
    # to the wall, the Sun's Song taught and its chain back, the song at the ReDeads.
    ("graveyard", S, ["--entrance", "ENTR_GRAVEYARD_0", "--child", "--preset", "graveyard-lullaby", "--time", "18:30", "--script", "graveyard", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("graveyard_dig", S, ["--entrance", "ENTR_GRAVEYARD_0", "--child", "--preset", "graveyard-lullaby", "--time", "18:30", "--script", "graveyard", "--frames", "880", "--screenshot", "{shot}"]),
    ("graveyard_tomb", S, ["--entrance", "ENTR_GRAVEYARD_0", "--child", "--preset", "graveyard-lullaby", "--time", "18:30", "--script", "graveyard", "--frames", "1950", "--screenshot", "{shot}"]),
    ("graveyard_keese", S, ["--entrance", "ENTR_GRAVEYARD_0", "--child", "--preset", "graveyard-lullaby", "--time", "18:30", "--script", "graveyard", "--frames", "3300", "--screenshot", "{shot}"]),
    ("graveyard_lesson", S, ["--entrance", "ENTR_GRAVEYARD_0", "--child", "--preset", "graveyard-lullaby", "--time", "18:30", "--script", "graveyard", "--frames", "5600", "--screenshot", "{shot}"]),
    # GAME-08 milestone 1: the meadow (Route::Meadow) from the Lost Woods' entrance with the letter: the
    # owl, the woods' right turns, the Wolfos killed and its gate sunk, three Mad Scrubs bounced and
    # killed, the maze and the ramp, Saria's lesson and Saria's Song.
    ("meadow", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "graveyard-sun", "--script", "meadow", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("meadow_woods", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "graveyard-sun", "--script", "meadow", "--frames", "1300", "--screenshot", "{shot}"]),
    ("meadow_wolfos", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "graveyard-sun", "--script", "meadow", "--frames", "2600", "--screenshot", "{shot}"]),
    ("meadow_scrub", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "graveyard-sun", "--script", "meadow", "--frames", "3850", "--screenshot", "{shot}"]),
    ("meadow_saria", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "graveyard-sun", "--script", "meadow", "--frames", "5700", "--screenshot", "{shot}"]),
    # GAME-08 milestone 2: the Lost Woods' Skull Kids and target scrub (Route::LostWoods) with Saria's
    # Song and the slingshot: Saria's Song on room 9's stump (his "You know Saria's Song"), the
    # target's bullseye (the score's 100), the target scrub up in its camera (4140), the memory
    # game's staff with the two Skull Kids on their stumps, and the third round's heart piece.
    ("lost_woods", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "meadow-saria", "--script", "lost-woods", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("lost_woods_stump", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "meadow-saria", "--script", "lost-woods", "--frames", "480", "--screenshot", "{shot}"]),
    ("lost_woods_target", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "meadow-saria", "--script", "lost-woods", "--frames", "1090", "--screenshot", "{shot}"]),
    ("lost_woods_scrub", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "meadow-saria", "--script", "lost-woods", "--frames", "1320", "--screenshot", "{shot}"]),
    ("lost_woods_game", S, ["--entrance", "ENTR_LOST_WOODS_0", "--child", "--preset", "meadow-saria", "--script", "lost-woods", "--frames", "2000", "--screenshot", "{shot}"]),
    # GAME-08 milestone 3, step (a): Lon Lon Ranch.
    ("ranch", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "lost-woods-games", "--script", "ranch", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("ranch_malon", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "lost-woods-games", "--script", "ranch", "--frames", "880", "--screenshot", "{shot}"]),
    ("ranch_song", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "lost-woods-games", "--script", "ranch", "--frames", "2000", "--screenshot", "{shot}"]),
    ("ranch_foal", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "lost-woods-games", "--script", "ranch", "--frames", "2290", "--screenshot", "{shot}"]),
    # GAME-08 milestone 3, step (b): Talon's house.
    ("ranch_cucco_game", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "lost-woods-games", "--script", "ranch", "--frames", "4700", "--screenshot", "{shot}"]),
    ("ranch_milk", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "lost-woods-games", "--script", "ranch", "--frames", "5600", "--screenshot", "{shot}"]),
    # GAME-09 milestone 1: Dodongo's Cavern's entrance with the bracelet and bombs (cavern-bombs):
    # the west Bomb Flower carried to the entrance wall, its blast, the cavern's opening; a bomb
    # out of the bag by the east flower.
    ("bombs", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-bombs", "--script", "bombs", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("bombs_carry", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-bombs", "--script", "bombs", "--frames", "150", "--screenshot", "{shot}"]),
    ("bombs_blast", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-bombs", "--script", "bombs", "--frames", "250", "--screenshot", "{shot}"]),
    ("bombs_opening", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-bombs", "--script", "bombs", "--frames", "330", "--screenshot", "{shot}"]),
    ("bombs_bomb", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-bombs", "--script", "bombs", "--frames", "505", "--screenshot", "{shot}"]),
    # GAME-09 milestone 2: Darunia (ranch-epona, Goron City's bottom floor): the lullaby at his
    # door, the door rising, Saria's Song and his dance, the bracelet; the boulder (goron-bracelet,
    # a debug start on the cliff above Dodongo's Cavern): the Bomb Flower thrown down onto it
    # (one-point 4180), its blast.
    ("darunia", S, ["--entrance", "ENTR_GORON_CITY_2", "--child", "--preset", "ranch-epona", "--script", "darunia", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("darunia_door", S, ["--entrance", "ENTR_GORON_CITY_2", "--child", "--preset", "ranch-epona", "--script", "darunia", "--frames", "300", "--screenshot", "{shot}"]),
    ("darunia_dance", S, ["--entrance", "ENTR_GORON_CITY_2", "--child", "--preset", "ranch-epona", "--script", "darunia", "--frames", "1300", "--screenshot", "{shot}"]),
    ("boulder", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_1", "--child", "--preset", "goron-bracelet", "--script", "boulder", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("boulder_throw", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_1", "--child", "--preset", "goron-bracelet", "--script", "boulder", "--frames", "110", "--screenshot", "{shot}"]),
    ("boulder_blast", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_1", "--child", "--preset", "goron-bracelet", "--script", "boulder", "--frames", "200", "--screenshot", "{shot}"]),
    # GAME-09 milestone 3: Dodongo's Cavern's runs, each a debug start. The island's flower, the pit's boulder and the
    # shortcut pillar to the upper level (cavern-lobby), the ride; the bomb bag's chest
    # (cavern-loop); the skull's eyes and jaw from the bridge (cavern-bomb-bag), the jaw opening;
    # the gravestone, door 20 and the fall to King Dodongo (cavern-jaw), the pull. And the sword's
    # trail (EffectBlure) mid-slash on the course.
    ("cavern_pillar", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-lobby", "--script", "pillar", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("cavern_pillar_ride", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-lobby", "--script", "pillar", "--frames", "700", "--screenshot", "{shot}"]),
    ("cavern_bomb_bag", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-loop", "--script", "bomb-bag", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("cavern_skull", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-bomb-bag", "--script", "skull", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("cavern_skull_jaw", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-bomb-bag", "--script", "skull", "--frames", "360", "--screenshot", "{shot}"]),
    ("cavern_gravestone", S, ["--entrance", "ENTR_DODONGOS_CAVERN_1", "--child", "--preset", "cavern-jaw", "--script", "gravestone", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("cavern_gravestone_pull", S, ["--entrance", "ENTR_DODONGOS_CAVERN_1", "--child", "--preset", "cavern-jaw", "--script", "gravestone", "--frames", "100", "--screenshot", "{shot}"]),
    ("course_sword_trail", S, ["--script", "sword", "--frames", "8", "--screenshot", "{shot}"]),
    # GAME-09 milestone 4: King Dodongo's room. The floor bombed, the drop and his intro (the trace;
    # his roar with the title card); after his defeat the heart, the warp and the Goron's Ruby's
    # cutscene to the trail's spawn 5 (the trace; the cleared room with the warp; Darunia giving
    # the Ruby).
    ("kingdodongo", S, ["--entrance", "ENTR_DODONGOS_CAVERN_BOSS_0", "--child", "--preset", "cavern-boss", "--script", "kingdodongo", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("kingdodongo_title", S, ["--entrance", "ENTR_DODONGOS_CAVERN_BOSS_0", "--child", "--preset", "cavern-boss", "--script", "kingdodongo", "--frames", "680", "--screenshot", "{shot}"]),
    ("ruby", S, ["--entrance", "ENTR_DODONGOS_CAVERN_BOSS_0", "--child", "--preset", "cavern-boss-beaten", "--script", "ruby", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("ruby_warp", S, ["--entrance", "ENTR_DODONGOS_CAVERN_BOSS_0", "--child", "--preset", "cavern-boss-beaten", "--script", "ruby", "--frames", "30", "--screenshot", "{shot}"]),
    ("ruby_jewel", S, ["--entrance", "ENTR_DODONGOS_CAVERN_BOSS_0", "--child", "--preset", "cavern-boss-beaten", "--script", "ruby", "--frames", "2400", "--screenshot", "{shot}"]),
    # GAME-09 milestone 6: the summit, the Great Fairy and magic.
    ("fairy_wall", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_4", "--child", "--preset", "goron-ruby", "--script", "fairy-wall", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("fairy_wall_blast", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_4", "--child", "--preset", "goron-ruby", "--script", "fairy-wall", "--frames", "108", "--screenshot", "{shot}"]),
    ("great_fairy", S, ["--entrance", "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", "--child", "--preset", "fairy-fountain", "--script", "great-fairy", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("great_fairy_appear", S, ["--entrance", "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", "--child", "--preset", "fairy-fountain", "--script", "great-fairy", "--frames", "650", "--screenshot", "{shot}"]),
    ("great_fairy_warp", S, ["--entrance", "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", "--child", "--preset", "fairy-fountain", "--script", "great-fairy", "--frames", "1250", "--screenshot", "{shot}"]),
    ("great_fairy_meter", S, ["--entrance", "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", "--child", "--preset", "fairy-fountain", "--script", "great-fairy", "--frames", "1950", "--screenshot", "{shot}"]),
    ("summit_owl", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_4", "--child", "--preset", "fairy-magic", "--script", "summit-owl", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("summit_owl_flight", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_4", "--child", "--preset", "fairy-magic", "--script", "summit-owl", "--frames", "800", "--screenshot", "{shot}"]),
    ("summit_spin", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_4", "--child", "--preset", "fairy-magic", "--at=-179,3314,-4365,5097", "--script", "spin", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("summit_spin_charge", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_4", "--child", "--preset", "fairy-magic", "--at=-179,3314,-4365,5097", "--script", "spin", "--frames", "70", "--screenshot", "{shot}"]),
    ("summit_spin_trail", S, ["--entrance", "ENTR_DEATH_MOUNTAIN_TRAIL_4", "--child", "--preset", "fairy-magic", "--at=-179,3314,-4365,5097", "--script", "spin", "--frames", "82", "--screenshot", "{shot}"]),
    # GAME-09 milestone 5: the rest of Dodongo's Cavern.
    ("falling_stairs", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "fairy-magic", "--script", "falling-stairs", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("falling_stairs_chain", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "fairy-magic", "--script", "falling-stairs", "--frames", "150", "--screenshot", "{shot}"]),
    ("falling_stairs_fall", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "fairy-magic", "--script", "falling-stairs", "--frames", "300", "--screenshot", "{shot}"]),
    ("silver_rupees", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "fairy-magic", "--script", "silver-rupees", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("silver_rupees_door", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "fairy-magic", "--script", "silver-rupees", "--frames", "160", "--screenshot", "{shot}"]),
    ("shield_chest", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-stairs", "--script", "shield-chest", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("shield_chest_pulled", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-stairs", "--script", "shield-chest", "--frames", "160", "--screenshot", "{shot}"]),
    ("time_blocks", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-song-of-time", "--script", "time-blocks", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("time_blocks_warp", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "cavern-song-of-time", "--script", "time-blocks", "--frames", "222", "--screenshot", "{shot}"]),
    # GAME-10 milestone 1: the bottle run (Zora's Domain's shallows): the milk drunk, a fish caught, let out.
    ("bottle", S, ["--entrance", "ENTR_ZORAS_DOMAIN_0", "--child", "--preset", "zora-bottle", "--script", "bottle", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("bottle_drink", S, ["--entrance", "ENTR_ZORAS_DOMAIN_0", "--child", "--preset", "zora-bottle", "--script", "bottle", "--frames", "90", "--screenshot", "{shot}"]),
    ("bottle_catch", S, ["--entrance", "ENTR_ZORAS_DOMAIN_0", "--child", "--preset", "zora-bottle", "--script", "bottle", "--frames", "560", "--screenshot", "{shot}"]),
    # GAME-10 milestones 2 and 3: the lullaby at Zora's River's falls into the Domain; Ruto's letter
    # to King Zora; a fish for Lord Jabu-Jabu.
    ("zoras_river", S, ["--entrance", "ENTR_ZORAS_RIVER_2", "--child", "--preset", "fairy-magic", "--script", "zoras-river", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("zoras_river_falls", S, ["--entrance", "ENTR_ZORAS_RIVER_2", "--child", "--preset", "fairy-magic", "--script", "zoras-river", "--frames", "320", "--screenshot", "{shot}"]),
    ("zoras_river_jump", S, ["--entrance", "ENTR_ZORAS_RIVER_2", "--child", "--preset", "fairy-magic", "--script", "zoras-river", "--frames", "406", "--screenshot", "{shot}"]),
    ("king_zora", S, ["--entrance", "ENTR_ZORAS_DOMAIN_1", "--child", "--preset", "zora-letter", "--script", "king-zora", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("king_zora_mweep", S, ["--entrance", "ENTR_ZORAS_DOMAIN_1", "--child", "--preset", "zora-letter", "--script", "king-zora", "--frames", "1000", "--screenshot", "{shot}"]),
    ("jabu_fish", S, ["--entrance", "ENTR_ZORAS_FOUNTAIN_2", "--child", "--preset", "jabu-fish", "--script", "jabu-fish", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("jabu_fish_inhale", S, ["--entrance", "ENTR_ZORAS_FOUNTAIN_2", "--child", "--preset", "jabu-fish", "--script", "jabu-fish", "--frames", "500", "--screenshot", "{shot}"]),
    ("cavern_dodongos", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "fairy-magic", "--room", "5", "--at=-1960,531,-650,0", "--script", "still", "--frames", "120", "--screenshot", "{shot}"]),
    ("cavern_beamos", S, ["--entrance", "ENTR_DODONGOS_CAVERN_0", "--child", "--preset", "fairy-magic", "--room", "2", "--at=-1645,0,-1242,-32768", "--script", "still", "--frames", "54", "--screenshot", "{shot}"]),
    ("ranch_guays", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "lost-woods-games", "--time", "22:00", "--at=-727,0,550,0", "--script", "still", "--frames", "420", "--trace", "{trace}", "--screenshot", "{shot}"]),
    ("ranch_ingo", S, ["--entrance", "ENTR_LON_LON_RANCH_0", "--child", "--preset", "castle-egg", "--at=760,0,-2040,-12000", "--script", "still", "--frames", "300", "--screenshot", "{shot}"]),
    ("stable_night", S, ["--entrance", "ENTR_STABLE_0", "--child", "--preset", "lost-woods-games", "--time", "22:00", "--at=300,0,60,-32768", "--script", "still", "--frames", "90", "--screenshot", "{shot}"]),
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
