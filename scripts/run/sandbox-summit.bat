@echo off
rem Headless: GAME-09 milestone 6's three scripted runs. Route::FairyWall (the summit in front of the
rem Great Fairy's wall, goron-ruby): a bomb breaks it, in through the hole. Route::GreatFairy (the
rem fountain, fairy-fountain): Zelda's Lullaby on the Triforce, her cutscene and the magic, out by the
rem return entrance to the trail's spawn 4. Route::SummitOwl (the summit with magic, fairy-magic): the
rem owl's talk and his flight to Kakariko.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEATH_MOUNTAIN_TRAIL_4 --child --preset goron-ruby --script fairy-wall ^
    --trace out\run\fairy_wall.json --screenshot out\run\fairy_wall.png --shots-at 30,60,100,110,140,215 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0 --child --preset fairy-fountain --script great-fairy ^
    --trace out\run\great_fairy.json --screenshot out\run\great_fairy.png --shots-at 30,150,300,500,700,900,1100,1300,1500,1700,1900,2100 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEATH_MOUNTAIN_TRAIL_4 --child --preset fairy-magic --script summit-owl ^
    --trace out\run\summit_owl.json --screenshot out\run\summit_owl.png --shots-at 30,100,200,400,600,800,1000,1200 %*
exit /b %errorlevel%
