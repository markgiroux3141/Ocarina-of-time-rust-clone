@echo off
rem Headless: GAME-09 milestone 3's four scripted runs, each a debug start of one of Dodongo's
rem Cavern's moments. Route::Pillar (the lobby's island, cavern-lobby): the island's flower thrown at the
rem pit's boulder, the switch, the shortcut pillar to the upper level. Route::BombBag (the east
rem corridor, cavern-loop): the big chest. Route::Skull (the bridge, cavern-bomb-bag): the bombs
rem equipped, one through each hole, the eyes and the jaw. Route::Gravestone (room 7's alcove,
rem cavern-jaw): the gravestone pulled off its switch, door 20, the fall to King Dodongo.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset cavern-lobby --script pillar ^
    --trace out\run\pillar.json --screenshot out\run\pillar.png --shots-at 60,150,190,250,316,433,582,700,800,819 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset cavern-loop --script bomb-bag ^
    --trace out\run\bomb-bag.json --screenshot out\run\bomb-bag.png --shots-at 30,120,200,300,358 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset cavern-bomb-bag --script skull ^
    --trace out\run\skull.json --screenshot out\run\skull.png --shots-at 40,63,100,130,217,250,300,360,470 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_1 --child --preset cavern-jaw --script gravestone ^
    --trace out\run\gravestone.json --screenshot out\run\gravestone.png --shots-at 60,125,222,300,385,447 %*
exit /b %errorlevel%
