@echo off
rem Headless: GAME-10 milestone 6's two scripted runs. Route::Barinade (the fall into his room,
rem jabu-barinade): his intro to the fight's start. Route::Sapphire (his room after his defeat,
rem barinade-beaten): the heart container, Ruto's warp, the Zora's Sapphire's cutscene at Zora's
rem Fountain.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_BOSS_0 --child --preset jabu-barinade --script barinade ^
    --trace out\run\barinade.json --screenshot out\run\barinade.png --shots-at 30,90,150,220,300,380,460,520,555 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_BOSS_0 --child --preset barinade-beaten --script sapphire ^
    --trace out\run\sapphire.json --screenshot out\run\sapphire.png --shots-at 60,300,535,590,700,760,913,1100,1400,1800,2200,2600,2768,2812 %*
exit /b %errorlevel%
