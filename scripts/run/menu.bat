@echo off
rem A menu of the scripts in this folder, for double-clicking. It comes back after each one.
setlocal
:menu
cls
echo OoT clone: run scripts (settings in scripts\run\_env.bat)
echo.
echo   Build and test
echo     1  build                Build everything (release)
echo     2  test                 All the tests
echo     3  test-playthrough     GAME-02's playthrough test
echo    11  test-sword-chest     GAME-03's Kokiri Sword chest test
echo    13  test-sword-route     GAME-03's crawlspace, boulder and sword route tests
echo    16  test-mido-shop       GAME-03's Mido, shop and Deku Shield route tests
echo    19  test-cutscenes       GAME-03's cutscene tests and the new save's run into the Deku Tree
echo    22  test-opening         GAME-03's Navi and opening tests, and the new file's run into the Deku Tree
echo    25  test-audio           GAME-04's audio tests: the microcode, a note, the reverb, Kokiri Forest's sequence
echo    28  test-music           GAME-04's music tests: Kokiri Forest's music and ambience as the game starts them
echo    31  test-sfx             GAME-04's sound effect tests: footsteps, the message box
echo    32  test-sfx-route       GAME-04's exit test: the runs' sound effects against the C
echo    36  test-onepoint        GAME-04b's one-point cutscene tests: the crawlspace's exits, attention, a chest's fall
echo    38  test-camera-modes    GAME-04b's jump, climb and hang camera tests
echo    39  test-cutscene-audio  GAME-04b's cutscene music, Link's cutscene voices and modes
echo    40  test-environment     GAME-04b's rain, lightning and light override tests
echo    41  test-title-cards     GAME-04b's title card tests: Kokiri Forest's place name, the Deku Tree's intro
echo    42  test-nightmare       GAME-04b's nightmare tests: the drawbridge, the riders, the horses, the rain
echo    43  test-polish          GAME-04b's polish tests: the cutscene splines, letterbox, narration box, Link's poses
echo     4  golden-check         The render and trace regression
echo     5  import               Re-import the asset pack
echo    44  decomp-check         GAME-05's decomp upgrade check: a loose import against the old decomp's
echo   113  coverage             The coverage page: the game's actors by phase, against what's ported
echo    45  test-damage          GAME-05's damage and health tests: the hit kinds, death and the game over, the Deku Baba
echo    50  test-combat          GAME-05's combat tests: the guard, the battle camera, the effects, the Keese, the withered Deku Baba
echo    53  test-enemies         GAME-05's other enemies' tests: the Deku Scrubs, the Skulltulas, Gohma's larvae
echo    56  test-mechanics       GAME-05's dungeon mechanics tests: the sliding doors, switches, torches, webs, quakes, the map
echo    59  test-sticks          GAME-05's Deku Stick and props tests: the stick, the platforms, the log, the crates, the lift
echo    62  test-push            GAME-05's push and pull tests: the push block, the gravestones, the time blocks, the rocks, room travel
echo    65  test-slingshot       GAME-05's slingshot and nut tests: first person, the seeds, the eye switches, room travel
echo    68  test-pause           GAME-05's pause menu tests: opening, the item page's cursor, the equip, the pages, closing
echo    71  test-dungeon-map     GAME-05's map page and game over tests: the floors, the room maps, the marks, the screens
echo    76  test-save            GAME-05's saving tests: z_sram.c, the save prompt, the game over's save, the reset and load
echo    80  test-gohma           GAME-05's Gohma tests: her intro, her eye's hits, her eggs, her death and decay, the heart
echo    83  test-blue-warp       GAME-05's blue warp tests: the warp, the float, the way out, the arrival by blue warp
echo    86  test-boss-room       GAME-05's last tests: room 9 into Gohma's room, the whole Deku Tree from one start
echo    89  test-emerald         GAME-06's Demo_Effect tests: the lights, the Triforce, the jewels, the time warp, the run
echo    92  test-creation        GAME-06's creation tests: Demo_Kankyo, the valley's bridges, the cloud ring, the sky, the chain
echo    95  test-farewell        GAME-06's farewell tests: Saria on the bridge, the soft soil, the owl, the run
echo    98  test-dusk            GAME-06's clock tests: time passing, the music, the sun, the lens flare, the run
echo   101  test-field           GAME-06's Hyrule Field tests: the Peahat, the Stalchildren, the enemy music, the runs
echo   104  test-market          GAME-07's Castle Town tests: Malon, the townsfolk, the cucco girl, the stones, the run
echo   107  test-castle          GAME-07's castle grounds tests: Talon, the crates, the sentries, the circles, the runs
echo   110  test-ocarina         GAME-07's ocarina tests: free play, the songs' effects, the fairy spot, a check, a lesson, the run
echo   113  test-zelda           GAME-07's courtyard and Zelda tests: the patrols, their look, Zelda, Impa, the run
echo   116  test-kakariko        GAME-07's Kakariko tests: the lift and carry, Anju, the people, the windmill, the run
echo   119  test-graveyard       GAME-07's graveyard tests: the ReDeads, Dampe, the kid, the tomb, the weather, the run
echo   122  test-meadow          GAME-08's meadow tests: Saria, the Wolfos, the gate, the run, the actor schemas
echo   126  test-lost-woods      GAME-08's Lost Woods tests: the Skull Kids, the memory game, the target scrub, the run
echo   129  test-ranch           GAME-08's ranch tests: the foal, the horses, Ingo, the Guays, Talon, the timers, the run
echo   132  test-bombs           GAME-09's bomb tests: the bomb, the Bomb Flower, the sparks, the blasts, the run
echo   135  test-goron           GAME-09's trail and Goron City tests: Gorons, Darunia, Tektites, the boulder, the urn, the runs
echo   138  test-cavern          GAME-09's cavern tests: the skull, Lizalfos, Poes, Armos, pillars, trails, the runs
echo   141  test-kingdodongo     GAME-09's King Dodongo tests: the boss, his breath, the lava, the Ruby's actors, the runs
echo   144  test-summit          GAME-09's summit tests: the magic meter, the Great Fairy, the spin, the rocks, the runs
echo   149  test-cavern-rooms    GAME-09's rest of the cavern: the stairs, silver rupees, Dodongos, Beamos, the rooms, the runs
echo   152  test-water           GAME-10's water and bottles: the dive, currents, the underwater view, bottles, fish, bugs, the run
echo.
echo   Play (a window; close it to come back)
echo     6  game                 Kokiri Forest
echo     7  game-links-house     From Link's bed, the Deku Tree's mouth open
echo     8  game-deku-tree-open  At the Deku Tree, his mouth open
echo     9  game-deku-tree-dead  At the Deku Tree after Gohma
echo    12  game-sword-chest     A new save at the Kokiri Sword's chest
echo    14  game-new-save        A new save from Link's bed (the sword, the shop, Mido)
echo    17  game-shop            Outside the Kokiri shop, the sword worn, 40 rupees
echo    20  game-deku-tree-talk  A new save at the Deku Tree: his talk (cutscenes), yes or no
echo    23  game-new-file        A new file as the file select starts it: the opening, then Navi
echo    26  game-music           Kokiri Forest with another sequence forced (the title theme)
echo    29  game-night           Kokiri Forest at night: its nature ambience
echo    33  game-door            A Kakariko house's door: its sounds, and Kakariko Village
echo    37  game-crawlspace      At the crawlspace's mouth: through it and out (its one-point cutscene)
echo    46  game-deku-baba       Inside the Deku Tree by a Deku Baba: its bite, its stem cut, death and the game over
echo    47  game-deku-baba-fairy The same with a fairy in a bottle: dying, it revives Link
echo    48  game-dummy           The training dummy, its touch hurting Link (asks which kind)
echo    51  game-combat          Inside the Deku Tree by a withered Deku Baba and a Keese: the shield (R), the battle camera
echo    54  game-enemies         Inside the Deku Tree by an enemy of milestone 3b (asks which)
echo    57  game-dungeon         Inside the Deku Tree at a room's debug start: the switch, the doors (asks where)
echo    60  game-sticks          Inside the Deku Tree with Deku Sticks on C-Left (J), by a torch (asks where)
echo    63  game-push            Inside the Deku Tree by room 3's push block, or room 7's or room 2's extras (asks where)
echo    66  game-slingshot       Inside the Deku Tree with the slingshot (L), nuts (K) and sticks (J), by an eye switch (asks where)
echo    69  game-pause           Inside the Deku Tree with the slingshot on no button: the pause menu (Enter) to equip it
echo    72  game-dungeon-map     Inside the Deku Tree with the compass: the pause menu's map page (Enter, then R)
echo    74  game-game-over       Inside the Deku Tree with a quarter heart by a Deku Baba: its bite, the game over's screens
echo    77  game-save            Inside the Deku Tree, a debug start in memory: equip, save (Enter, E, Space), F5 loads it
echo    79  game-file            File 2 of your save file (out\saves): game-file.bat N [--new-file]
echo    81  game-gohma           Queen Gohma's room: her intro, the fight (asks: first try or again)
echo    84  game-blue-warp       Queen Gohma's room after her defeat, before the blue warp: into it, out to the Deku Tree
echo    87  game-boss-room       The Deku Tree's room 9, cleared: through room 11 into Gohma's room, the fight, the warp
echo    90  game-emerald         Kokiri Forest's cutscene layer 6: the emerald, the tree's death (asks: or Farore's)
echo    93  game-creation        The creation from the cutscene map, part by part to the emerald (asks: or one part)
echo    96  game-farewell        Kokiri Forest after the emerald: Mido, the bridge, Saria, Hyrule Field, the owl (asks: or later)
echo    99  game-dusk            Hyrule Field at 17:00: the sunset, nightfall, the drawbridge (asks: or night, or dawn)
echo   102  game-field           Hyrule Field by day by a Peahat (asks: or night, the Stalchildren)
echo   105  game-market          Castle Town's entrance by day (asks: or night, the temple, the altar, the alley)
echo   108  game-castle          Hyrule Castle's road with the Cucco (asks: or by Talon, the egg, night, dawn, Kakariko)
echo   111  game-ocarina         Hyrule Field at 18:30 with the ocarina and six songs (asks: or by day, night, the fairy spot)
echo   114  game-zelda           The guards' courtyard from the drain (asks: or night, Zelda's courtyard, Impa, the field)
echo   117  game-kakariko        Kakariko by day with the letter (asks: or the pen, the roof, the gate, night, an interior)
echo   120  game-graveyard       The graveyard at 18:30 with the lullaby (asks: or day, Dampe, the tomb, inside, the ReDeads, a grave)
echo   123  game-meadow          The Lost Woods to the meadow with the letter (asks: or the meadow, Saria, a new save's greeting)
echo   127  game-lost-woods      The Lost Woods' Skull Kids and target with Saria's Song (asks: or the stump, the target, the game)
echo   130  game-ranch           Lon Lon Ranch: Malon, the foal, Talon's cucco game (asks: or Malon, night, the stable, Ingo, the house)
echo   133  game-bombs           Dodongo's Cavern's entrance with the bracelet and bombs (asks: or Hyrule Field)
echo   136  game-goron           Death Mountain Trail and Goron City (asks: or city, darunia, cliff, cavern, bombs)
echo   139  game-cavern          Dodongo's Cavern, the whole loop (asks: or lizalfos, pillar, upper, web, bag, skull, gravestone, boss)
echo   142  game-kingdodongo     King Dodongo's room (asks: or fight, beaten, ruby, trail)
echo   145  game-summit          Death Mountain's summit and the Great Fairy (asks: or rocks, wall, fountain, revisit, owl)
echo   147  editor               The game editor: levels, props and the game's actors; Play runs them in the game
echo   148  editor-actors        The actors as the editor draws them: captured from the game, a contact sheet of them all
echo   150  game-cavern-rooms    Dodongo's Cavern's rest from room 2 (asks: or stairs, rupees, beamos, dodongos, larvae, scrubs, poe, blocks)
echo   153  game-water           Zora's Domain's shallows with milk and the fish (asks: or lake, river, tunnel)
echo   155  game-zora            The diving game (asks: or shop, king, jabu, lake, lab, river, boomerang)
echo.
echo   Headless
echo    10  sandbox-playthrough  The playthrough's trace and screenshots, into out\run
echo    15  sandbox-sword-chest  The sword route's trace and screenshots, into out\run
echo    18  sandbox-mido-shop    The shop and Mido route's trace and screenshots, into out\run
echo    21  sandbox-new-save-deku-tree  The new save's run into the Deku Tree, into out\run
echo    24  sandbox-new-file-deku-tree  The new file's run: the opening, Navi, the Deku Tree, into out\run
echo    27  audio-wav            WAVs of Kokiri Forest's music, a note and a drum, into out\audio (opens the music)
echo    30  sandbox-audio-log    The new file's run with its audio log and WAV, into out\run (opens the WAV)
echo    34  sandbox-mido-shop-audio  The Mido and shop run's audio log and WAV, into out\run (opens the WAV)
echo    35  ootx-sfx             Look a sound effect up by id or name
echo    49  sandbox-deku-baba    The Deku Baba run's trace and screenshots, into out\run
echo    52  sandbox-combat       The combat run's trace and screenshots, into out\run
echo    55  sandbox-scrub        The Mad Scrub run's trace and screenshots, into out\run
echo    58  sandbox-shutter      The switch and sliding door run's trace and screenshots, into out\run
echo    61  sandbox-stick        The Deku Stick run's trace and screenshots (the torch, the web, room 1), into out\run
echo    64  sandbox-push         The push block run's trace and screenshots (pushed into the pit, climbed), into out\run
echo    67  sandbox-slingshot    The slingshot run's trace and screenshots (the eye shot, the door, room 2), into out\run
echo    70  sandbox-pause        The pause menu run's trace and screenshots (the item page, the equip, the map page), into out\run
echo    73  sandbox-dungeon-map  The map page run's trace and screenshots (1F, then 2F), into out\run
echo    75  sandbox-game-over    The game over run's trace and screenshots (the message, the prompts), into out\run
echo    78  sandbox-save         The save run's trace, screenshots and SRAM image (saved, reset, loaded), into out\run
echo    82  sandbox-gohma        The Gohma run's trace and screenshots (the card, the stun, her decay, the heart), into out\run
echo    85  sandbox-blue-warp    The blue warp run's trace and screenshots (the warp, the float, the arrival), into out\run
echo    88  sandbox-boss-room    The room 9 to Gohma's room run's trace and screenshots, into out\run
echo    91  sandbox-emerald      The emerald run's trace and screenshots (the light, the emerald over Link), into out\run
echo    94  sandbox-creation     The creation run's trace and screenshots (the rain, the rocks, the rings), into out\run
echo    97  sandbox-farewell     The farewell run's trace and screenshots (Saria, the ocarina, the owl), into out\run
echo   100  sandbox-dusk         The dusk run's trace and screenshots (the sunset, the bridge, the night), into out\run
echo   103  sandbox-field        The field runs' traces and screenshots (the Peahat, the Stalchildren), into out\run
echo   106  sandbox-market       The market run's trace and screenshots (the townsman, Malon, the castle), into out\run
echo   109  sandbox-castle       The castle runs' traces and screenshots (Talon, the crates, the drain, caught), into out\run
echo   112  sandbox-ocarina      The ocarina run's trace and screenshots (the staff, Saria's, the sun, the storm), into out\run
echo   115  sandbox-zelda        The Zelda run's trace and screenshots (the patrols, Zelda, Ganondorf, Impa), into out\run
echo   118  sandbox-kakariko     The Kakariko run's trace and screenshots (the cuccos, the glide, the gate), into out\run
echo   121  sandbox-graveyard    The graveyard run's trace and screenshots (Dampe, the tomb, the Keese, the Sun's Song), into out\run
echo   124  sandbox-meadow       The meadow run's trace and screenshots (the woods, the Wolfos, the scrubs, Saria), into out\run
echo   125  export-glb           The game's characters as .glb files for Blender, T-posed, into out\glb\characters
echo   128  sandbox-lost-woods   The Lost Woods run's trace and screenshots (the stump, the target, the memory game), into out\run
echo   131  sandbox-ranch        The ranch run's trace and screenshots (Malon, the foal, the cucco game), into out\run
echo   134  sandbox-bombs        The bombs run's trace and screenshots (the flower, the wall, the bomb), into out\run
echo   137  sandbox-goron        The Darunia and boulder runs' traces and screenshots, into out\run
echo   140  sandbox-cavern       The cavern's four runs' traces and screenshots (pillar, bag, skull, gravestone), into out\run
echo   143  sandbox-kingdodongo  King Dodongo's two runs' traces and screenshots (the intro, the Ruby), into out\run
echo   146  sandbox-summit       The summit's three runs' traces and screenshots (the wall, the fairy, the owl), into out\run
echo   151  sandbox-cavern-rooms The cavern's rest's four runs' traces and screenshots (stairs, rupees, chest, blocks), into out\run
echo   154  sandbox-water        The bottle run's trace and screenshots (the milk, the fish caught and let out), into out\run
echo   156  sandbox-zora-runs    The river, King Zora and Jabu-Jabu runs' traces and screenshots, into out\run
echo   157  game-jabu            Jabu-Jabu's room 0 (asks: or elevator, flobbery, pit, spout, platform, room2, room14, boomerang, door14, ruto, carry, stick, barriers, gray, red, green, crates, crystal, room5, door0)
echo   158  test-jabu            Jabu-Jabu's draw config, switches, lifts, spouts, enemies, Ruto, tentacles and runs (cargo test)
echo   159  sandbox-jabu-runs    The Jabu-Jabu runs' traces and screenshots (the cow, the elevator, the boomerang, door 14, Ruto, the web, the crystal), into out\run
echo   160  game-barinade        Barinade's room (asks: or fight, beaten, fountain, sapphire)
echo   161  test-barinade        Barinade, Ruto's warp, the Sapphire's cutscene and the runs (cargo test)
echo   162  sandbox-barinade     The Barinade and Sapphire runs' traces and screenshots, into out\run
echo   163  game-ocarina-of-time Hyrule Field short of the drawbridge (asks: or escape, moat, temple, field)
echo   164  test-ocarina-of-time The Ocarina of Time, Ganondorf's cape and the runs (cargo test)
echo   165  sandbox-ocarina-of-time The escape and Ocarina of Time runs' traces and screenshots, into out\run
echo   166  game-frogs           Zora's River on the frogs' log (asks: or choir, lake, longsong, bonooru, pierre)
echo   167  test-frogs           The frogs, the scarecrows, the ocarina's recording and the runs (cargo test)
echo   168  sandbox-frogs        The frogs' and Bonooru's runs' traces and screenshots, into out\run
echo   169  game-master-sword    The Temple of Time's pedestal, a child (asks: or back, door)
echo   170  test-master-sword    The age switch, the pedestal, time travel and the runs (cargo test)
echo   171  sandbox-master-sword The pull's and the way back's runs' traces and screenshots, into out\run
echo   172  game-temple-of-time  The Door of Time to the Chamber and out (asks: or pull, chamber, awakening, temple, outside, market, field)
echo   173  test-temple-of-time  The door, the first pull, the Chamber, Sheik, the ruins and Bg_Spot00_Break (cargo test)
echo   174  sandbox-temple-of-time The five runs' traces and screenshots, into out\run
echo   175  game-prelude         Sheik's Prelude, the sword put back and pulled again, the warp (asks: or back, again, warp)
echo   176  test-prelude         The Prelude, time travel both ways and the warp's runs (cargo test)
echo   177  sandbox-prelude      The four runs' traces and screenshots, into out\run
echo   178  game-epona           Riding Epona: her song, on her, a fence jumped (asks: or field, ranch, night)
echo   179  test-epona           The riding runs, En_Horse, Bg_Umajump and the horse camera (cargo test)
echo   180  sandbox-epona        The two riding runs' traces and screenshots, into out\run
echo.
echo     0  quit (or an empty line)
echo.
set "pick="
rem An empty line quits (set /p fails on it, and at the end of piped input).
set /p "pick=Choose: " || exit /b 0
set "pick=%pick: =%"
if "%pick%"=="0" exit /b 0
set "script="
if "%pick%"=="1" set "script=build"
if "%pick%"=="2" set "script=test"
if "%pick%"=="3" set "script=test-playthrough"
if "%pick%"=="4" set "script=golden-check"
if "%pick%"=="5" set "script=import"
if "%pick%"=="6" set "script=game"
if "%pick%"=="7" set "script=game-links-house"
if "%pick%"=="8" set "script=game-deku-tree-open"
if "%pick%"=="9" set "script=game-deku-tree-dead"
if "%pick%"=="10" set "script=sandbox-playthrough"
if "%pick%"=="11" set "script=test-sword-chest"
if "%pick%"=="12" set "script=game-sword-chest"
if "%pick%"=="13" set "script=test-sword-route"
if "%pick%"=="14" set "script=game-new-save"
if "%pick%"=="15" set "script=sandbox-sword-chest"
if "%pick%"=="16" set "script=test-mido-shop"
if "%pick%"=="17" set "script=game-shop"
if "%pick%"=="18" set "script=sandbox-mido-shop"
if "%pick%"=="19" set "script=test-cutscenes"
if "%pick%"=="20" set "script=game-deku-tree-talk"
if "%pick%"=="21" set "script=sandbox-new-save-deku-tree"
if "%pick%"=="22" set "script=test-opening"
if "%pick%"=="23" set "script=game-new-file"
if "%pick%"=="24" set "script=sandbox-new-file-deku-tree"
if "%pick%"=="25" set "script=test-audio"
if "%pick%"=="26" set "script=game-music"
if "%pick%"=="27" set "script=audio-wav"
if "%pick%"=="28" set "script=test-music"
if "%pick%"=="29" set "script=game-night"
if "%pick%"=="30" set "script=sandbox-audio-log"
if "%pick%"=="31" set "script=test-sfx"
if "%pick%"=="32" set "script=test-sfx-route"
if "%pick%"=="33" set "script=game-door"
if "%pick%"=="34" set "script=sandbox-mido-shop-audio"
if "%pick%"=="35" set "script=ootx-sfx"
if "%pick%"=="36" set "script=test-onepoint"
if "%pick%"=="37" set "script=game-crawlspace"
if "%pick%"=="38" set "script=test-camera-modes"
if "%pick%"=="39" set "script=test-cutscene-audio"
if "%pick%"=="40" set "script=test-environment"
if "%pick%"=="41" set "script=test-title-cards"
if "%pick%"=="42" set "script=test-nightmare"
if "%pick%"=="43" set "script=test-polish"
if "%pick%"=="44" set "script=decomp-check"
if "%pick%"=="113" set "script=coverage"
if "%pick%"=="45" set "script=test-damage"
if "%pick%"=="46" set "script=game-deku-baba"
if "%pick%"=="47" goto fairy
if "%pick%"=="48" goto dummy
if "%pick%"=="49" set "script=sandbox-deku-baba"
if "%pick%"=="50" set "script=test-combat"
if "%pick%"=="51" set "script=game-combat"
if "%pick%"=="52" set "script=sandbox-combat"
if "%pick%"=="53" set "script=test-enemies"
if "%pick%"=="54" goto enemies
if "%pick%"=="55" set "script=sandbox-scrub"
if "%pick%"=="56" set "script=test-mechanics"
if "%pick%"=="57" goto dungeon
if "%pick%"=="58" set "script=sandbox-shutter"
if "%pick%"=="59" set "script=test-sticks"
if "%pick%"=="60" goto sticks
if "%pick%"=="61" set "script=sandbox-stick"
if "%pick%"=="62" set "script=test-push"
if "%pick%"=="63" goto push
if "%pick%"=="64" set "script=sandbox-push"
if "%pick%"=="65" set "script=test-slingshot"
if "%pick%"=="66" goto slingshot
if "%pick%"=="67" set "script=sandbox-slingshot"
if "%pick%"=="68" set "script=test-pause"
if "%pick%"=="69" set "script=game-pause"
if "%pick%"=="70" set "script=sandbox-pause"
if "%pick%"=="71" set "script=test-dungeon-map"
if "%pick%"=="72" set "script=game-dungeon-map"
if "%pick%"=="73" set "script=sandbox-dungeon-map"
if "%pick%"=="74" set "script=game-game-over"
if "%pick%"=="75" set "script=sandbox-game-over"
if "%pick%"=="76" set "script=test-save"
if "%pick%"=="77" set "script=game-save"
if "%pick%"=="78" set "script=sandbox-save"
if "%pick%"=="79" set "script=game-file"
if "%pick%"=="80" set "script=test-gohma"
if "%pick%"=="81" goto gohma
if "%pick%"=="82" set "script=sandbox-gohma"
if "%pick%"=="83" set "script=test-blue-warp"
if "%pick%"=="84" set "script=game-blue-warp"
if "%pick%"=="85" set "script=sandbox-blue-warp"
if "%pick%"=="86" set "script=test-boss-room"
if "%pick%"=="87" set "script=game-boss-room"
if "%pick%"=="88" set "script=sandbox-boss-room"
if "%pick%"=="89" set "script=test-emerald"
if "%pick%"=="90" goto emerald
if "%pick%"=="91" set "script=sandbox-emerald"
if "%pick%"=="92" set "script=test-creation"
if "%pick%"=="93" goto creation
if "%pick%"=="94" set "script=sandbox-creation"
if "%pick%"=="95" set "script=test-farewell"
if "%pick%"=="96" goto farewell
if "%pick%"=="97" set "script=sandbox-farewell"
if "%pick%"=="98" set "script=test-dusk"
if "%pick%"=="99" goto dusk
if "%pick%"=="100" set "script=sandbox-dusk"
if "%pick%"=="101" set "script=test-field"
if "%pick%"=="102" goto field
if "%pick%"=="103" set "script=sandbox-field"
if "%pick%"=="104" set "script=test-market"
if "%pick%"=="105" goto market
if "%pick%"=="106" set "script=sandbox-market"
if "%pick%"=="107" set "script=test-castle"
if "%pick%"=="108" goto castle
if "%pick%"=="109" set "script=sandbox-castle"
if "%pick%"=="110" set "script=test-ocarina"
if "%pick%"=="111" goto ocarina
if "%pick%"=="112" set "script=sandbox-ocarina"
if "%pick%"=="113" set "script=test-zelda"
if "%pick%"=="114" goto zelda
if "%pick%"=="115" set "script=sandbox-zelda"
if "%pick%"=="116" set "script=test-kakariko"
if "%pick%"=="117" goto kakariko
if "%pick%"=="118" set "script=sandbox-kakariko"
if "%pick%"=="119" set "script=test-graveyard"
if "%pick%"=="120" goto graveyard
if "%pick%"=="121" set "script=sandbox-graveyard"
if "%pick%"=="122" set "script=test-meadow"
if "%pick%"=="123" goto meadow
if "%pick%"=="124" set "script=sandbox-meadow"
if "%pick%"=="125" set "script=export-glb"
if "%pick%"=="126" set "script=test-lost-woods"
if "%pick%"=="127" goto lostwoods
if "%pick%"=="128" set "script=sandbox-lost-woods"
if "%pick%"=="129" set "script=test-ranch"
if "%pick%"=="130" goto ranch
if "%pick%"=="131" set "script=sandbox-ranch"
if "%pick%"=="132" set "script=test-bombs"
if "%pick%"=="133" goto bombs
if "%pick%"=="134" set "script=sandbox-bombs"
if "%pick%"=="135" set "script=test-goron"
if "%pick%"=="136" goto goron
if "%pick%"=="137" set "script=sandbox-goron"
if "%pick%"=="138" set "script=test-cavern"
if "%pick%"=="139" goto cavern
if "%pick%"=="140" set "script=sandbox-cavern"
if "%pick%"=="141" set "script=test-kingdodongo"
if "%pick%"=="142" goto kingdodongo
if "%pick%"=="143" set "script=sandbox-kingdodongo"
if "%pick%"=="144" set "script=test-summit"
if "%pick%"=="145" goto summit
if "%pick%"=="146" set "script=sandbox-summit"
if "%pick%"=="147" set "script=editor"
if "%pick%"=="148" set "script=editor-actors"
if "%pick%"=="149" set "script=test-cavern-rooms"
if "%pick%"=="150" goto cavernrooms
if "%pick%"=="151" set "script=sandbox-cavern-rooms"
if "%pick%"=="152" set "script=test-water"
if "%pick%"=="153" goto water
if "%pick%"=="154" set "script=sandbox-water"
if "%pick%"=="155" goto zora
if "%pick%"=="156" set "script=sandbox-zora-runs"
if "%pick%"=="157" goto jabu
if "%pick%"=="158" set "script=test-jabu"
if "%pick%"=="159" set "script=sandbox-jabu-runs"
if "%pick%"=="160" goto barinade
if "%pick%"=="161" set "script=test-barinade"
if "%pick%"=="162" set "script=sandbox-barinade"
if "%pick%"=="163" goto ocarinaoftime
if "%pick%"=="164" set "script=test-ocarina-of-time"
if "%pick%"=="165" set "script=sandbox-ocarina-of-time"
if "%pick%"=="166" goto frogs
if "%pick%"=="167" set "script=test-frogs"
if "%pick%"=="168" set "script=sandbox-frogs"
if "%pick%"=="169" goto mastersword
if "%pick%"=="170" set "script=test-master-sword"
if "%pick%"=="171" set "script=sandbox-master-sword"
if "%pick%"=="172" goto templeoftime
if "%pick%"=="173" set "script=test-temple-of-time"
if "%pick%"=="174" set "script=sandbox-temple-of-time"
if "%pick%"=="175" goto prelude
if "%pick%"=="176" set "script=test-prelude"
if "%pick%"=="177" set "script=sandbox-prelude"
if "%pick%"=="178" goto epona
if "%pick%"=="179" set "script=test-epona"
if "%pick%"=="180" set "script=sandbox-epona"
if not defined script goto menu
echo.
call "%~dp0%script%.bat"
echo.
echo %script% finished (exit code %errorlevel%).
pause
goto menu

:kakariko
set "which="
set /p "which=Which (empty: the entrance by day; pen, roof, gate, night, dusk, cow, windmill, skulltula, guests, shop or guardhouse): "
call "%~dp0game-kakariko.bat" %which%
echo game-kakariko finished (exit code %errorlevel%).
pause
goto menu

:cavern
set "which="
set /p "which=Which (empty: the cavern's entrance; lizalfos, pillar, upper, web, bag, skull, gravestone or boss): "
call "%~dp0game-cavern.bat" %which%
echo game-cavern finished (exit code %errorlevel%).
pause
goto menu

:cavernrooms
set "which="
set /p "which=Which (empty: room 2 by its south door; stairs, rupees, beamos, dodongos, larvae, scrubs, poe or blocks): "
call "%~dp0game-cavern-rooms.bat" %which%
echo game-cavern-rooms finished (exit code %errorlevel%).
pause
goto menu

:water
set "which="
set /p "which=Which (empty: Zora's Domain's shallows; lake, river or tunnel): "
call "%~dp0game-water.bat" %which%
echo game-water finished (exit code %errorlevel%).
pause
goto menu

:zora
set "which="
set /p "which=Which (empty: the diving game; shop, king, jabu, lake, lab, river or boomerang): "
call "%~dp0game-zora.bat" %which%
echo game-zora finished (exit code %errorlevel%).
pause
goto menu

:jabu
set "which="
set /p "which=Which (empty: room 0; elevator, flobbery, pit, spout, platform, room2, room14, boomerang, door14, ruto, carry, stick, barriers, gray, red, green, crates, crystal, room5 or door0): "
call "%~dp0game-jabu.bat" %which%
echo game-jabu finished (exit code %errorlevel%).
pause
goto menu

:epona
set "which="
set /p "which=Which (empty: the ranch's corral before Epona; field, ranch or night): "
call "%~dp0game-epona.bat" %which%
echo game-epona finished (exit code %errorlevel%).
pause
goto menu

:prelude
set "which="
set /p "which=Which (empty: the Temple's entrance, Sheik's Prelude; back, again or warp): "
call "%~dp0game-prelude.bat" %which%
echo game-prelude finished (exit code %errorlevel%).
pause
goto menu

:templeoftime
set "which="
set /p "which=Which (empty: the Temple's entrance, the door shut; pull, chamber, awakening, temple, outside, market or field): "
call "%~dp0game-temple-of-time.bat" %which%
echo game-temple-of-time finished (exit code %errorlevel%).
pause
goto menu

:mastersword
set "which="
set /p "which=Which (empty: a child at the pedestal; back or door): "
call "%~dp0game-master-sword.bat" %which%
echo game-master-sword finished (exit code %errorlevel%).
pause
goto menu

:frogs
set "which="
set /p "which=Which (empty: the frogs' log; choir, lake, longsong, bonooru or pierre): "
call "%~dp0game-frogs.bat" %which%
echo game-frogs finished (exit code %errorlevel%).
pause
goto menu

:ocarinaoftime
set "which="
set /p "which=Which (empty: the field short of the drawbridge; escape, moat, temple or field): "
call "%~dp0game-ocarina-of-time.bat" %which%
echo game-ocarina-of-time finished (exit code %errorlevel%).
pause
goto menu

:barinade
set "which="
set /p "which=Which (empty: the fall into his room; fight, beaten, fountain or sapphire): "
call "%~dp0game-barinade.bat" %which%
echo game-barinade finished (exit code %errorlevel%).
pause
goto menu

:summit
set "which="
set /p "which=Which (empty: the trail by Goron City's door; rocks, wall, fountain, revisit or owl): "
call "%~dp0game-summit.bat" %which%
echo game-summit finished (exit code %errorlevel%).
pause
goto menu

:kingdodongo
set "which="
set /p "which=Which (empty: the room's spawn; fight, beaten, ruby or trail): "
call "%~dp0game-kingdodongo.bat" %which%
echo game-kingdodongo finished (exit code %errorlevel%).
pause
goto menu

:goron
set "which="
set /p "which=Which (empty: the trail's foot; city, darunia, cliff, cavern or bombs): "
call "%~dp0game-goron.bat" %which%
echo game-goron finished (exit code %errorlevel%).
pause
goto menu

:bombs
set "which="
set /p "which=Which (empty: Dodongo's Cavern's entrance; field): "
call "%~dp0game-bombs.bat" %which%
echo game-bombs finished (exit code %errorlevel%).
pause
goto menu

:ranch
set "which="
set /p "which=Which (empty: the ranch's gate; malon, night, stable, stable-night, ingo or house): "
call "%~dp0game-ranch.bat" %which%
echo game-ranch finished (exit code %errorlevel%).
pause
goto menu

:lostwoods
set "which="
set /p "which=Which (empty: the Lost Woods from Kokiri Forest; stump, target or game): "
call "%~dp0game-lost-woods.bat" %which%
echo game-lost-woods finished (exit code %errorlevel%).
pause
goto menu

:meadow
set "which="
set /p "which=Which (empty: the Lost Woods from Kokiri Forest; meadow, saria or kokiri): "
call "%~dp0game-meadow.bat" %which%
echo game-meadow finished (exit code %errorlevel%).
pause
goto menu

:graveyard
set "which="
set /p "which=Which (empty: the entrance at 18:30; day, dampe, tomb, inside, redeads, redead, shield or hut): "
call "%~dp0game-graveyard.bat" %which%
echo game-graveyard finished (exit code %errorlevel%).
pause
goto menu

:zelda
set "which="
set /p "which=Which (empty: the guards' courtyard from the drain; night, zelda, impa or field): "
call "%~dp0game-zelda.bat" %which%
echo game-zelda finished (exit code %errorlevel%).
pause
goto menu

:ocarina
set "which="
set /p "which=Which (empty: 18:30 north of the fairy spot; day, night or fairy): "
call "%~dp0game-ocarina.bat" %which%
echo game-ocarina finished (exit code %errorlevel%).
pause
goto menu

:castle
set "which="
set /p "which=Which (empty: the road with the Cucco; talon, egg, night, dawn or kakariko): "
call "%~dp0game-castle.bat" %which%
echo game-castle finished (exit code %errorlevel%).
pause
goto menu

:market
set "which="
set /p "which=Which (empty: the entrance by day; night, temple, altar or alley): "
call "%~dp0game-market.bat" %which%
echo game-market finished (exit code %errorlevel%).
pause
goto menu

:field
set "which="
set /p "which=Which (empty: by day, by the Peahat; night): "
call "%~dp0game-field.bat" %which%
echo game-field finished (exit code %errorlevel%).
pause
goto menu

:dusk
set "which="
set /p "which=Which (empty: 17:00; night or dawn): "
call "%~dp0game-dusk.bat" %which%
echo game-dusk finished (exit code %errorlevel%).
pause
goto menu

:farewell
set "which="
set /p "which=Which (empty: Kokiri Forest after the emerald; bridge or field): "
call "%~dp0game-farewell.bat" %which%
echo game-farewell finished (exit code %errorlevel%).
pause
goto menu

:creation
set "which="
set /p "which=Which (empty: from the start; goddesses, din, valley, nayru or triforce): "
call "%~dp0game-creation.bat" %which%
echo game-creation finished (exit code %errorlevel%).
pause
goto menu

:emerald
set "which="
set /p "which=Which (empty: the emerald, layer 6; farore: layer 4): "
call "%~dp0game-emerald.bat" %which%
echo game-emerald finished (exit code %errorlevel%).
pause
goto menu

:fairy
call "%~dp0game-deku-baba.bat" fairy
echo game-deku-baba fairy finished (exit code %errorlevel%).
pause
goto menu

:dummy
set "kind="
set /p "kind=Kind (none, fire, ice, electric, knockback): "
call "%~dp0game-dummy.bat" %kind%
echo game-dummy finished (exit code %errorlevel%).
pause
goto menu

:enemies
set "who="
set /p "who=Enemy (scrub, hint, shop, skulltula, walltula, gold, larva): "
call "%~dp0game-enemies.bat" %who%
echo game-enemies finished (exit code %errorlevel%).
pause
goto menu

:dungeon
set "where="
set /p "where=Where (switch, lobby-top, lobby, room1 to room10): "
call "%~dp0game-dungeon.bat" %where%
echo game-dungeon finished (exit code %errorlevel%).
pause
goto menu

:sticks
set "where="
set /p "where=Where (torch, room3, room10, room5, room2): "
call "%~dp0game-sticks.bat" %where%
echo game-sticks finished (exit code %errorlevel%).
pause
goto menu

:push
set "where="
set /p "where=Where (room3, room7, room2): "
call "%~dp0game-push.bat" %where%
echo game-push finished (exit code %errorlevel%).
pause
goto menu

:gohma
set "try="
set /p "try=First try or again (empty or again): "
call "%~dp0game-gohma.bat" %try%
echo game-gohma finished (exit code %errorlevel%).
pause
goto menu

:slingshot
set "where="
set /p "where=Where (room1, room3, room2, room10): "
call "%~dp0game-slingshot.bat" %where%
echo game-slingshot finished (exit code %errorlevel%).
pause
goto menu
