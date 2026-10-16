@echo off
rem Headless: GAME-09 milestone 2's two scripted runs. Route::Darunia from Goron City's bottom floor
rem (ranch-epona): Zelda's Lullaby at the spot by Darunia's door, the door, Saria's Song to him,
rem his dance and the bracelet. Route::Boulder from a debug start on the cliff above Dodongo's
rem Cavern (goron-bracelet): the Bomb Flower lifted and thrown down onto the boulder, its blast.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_GORON_CITY_2 --child --preset ranch-epona --script darunia ^
    --trace out\run\darunia.json --screenshot out\run\darunia.png --shots-at 100,300,450,900,1300,2200 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEATH_MOUNTAIN_TRAIL_1 --child --preset goron-bracelet --script boulder ^
    --trace out\run\boulder.json --screenshot out\run\boulder.png --shots-at 30,60,110,160,200,240 %*
exit /b %errorlevel%
