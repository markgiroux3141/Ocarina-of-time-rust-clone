@echo off
rem Headless: GAME-09 milestone 5's four scripted runs (no fights: those are the hand test's).
rem Route::FallingStairs (room 2 south of its Bomb Flowers, fairy-magic): a bomb at the rows'
rem junction, the chain, one-point 3050 and the stairs down. Route::SilverRupees (room 2's balcony,
rem four rupees taken, the room cleared): the last one's crate bombed, the rupee, door 17's camera.
rem Route::ShieldChest (room 14 by its gravestone, the room cleared, cavern-stairs): the gravestone
rem pulled and the chest opened. Route::TimeBlocks (room 15 by its door, cavern-song-of-time): the
rem Song of Time, the nearest block gone.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset fairy-magic --script falling-stairs ^
    --trace out\run\falling_stairs.json --screenshot out\run\falling_stairs.png --shots-at 30,60,120,180,240,300,360,420 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset fairy-magic --script silver-rupees ^
    --trace out\run\silver_rupees.json --screenshot out\run\silver_rupees.png --shots-at 30,90,150,200,240,280 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset cavern-stairs --script shield-chest ^
    --trace out\run\shield_chest.json --screenshot out\run\shield_chest.png --shots-at 30,100,150,220,280 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset cavern-song-of-time --script time-blocks ^
    --trace out\run\time_blocks.json --screenshot out\run\time_blocks.png --shots-at 30,60,95,150,200,235 %*
exit /b %errorlevel%
