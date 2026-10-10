@echo off
rem Headless: GAME-06 milestone 3's scripted run: Hyrule Field from 17:00 in front of the
rem drawbridge (ENTR_HYRULE_FIELD_0, deku-tree-dead): west into the sunset while the music stops,
rem round to the castle at nightfall as the drawbridge rises, until the night's critters.
rem Writes its trace and screenshots to out\run\: the sunset and its lens flare, the bridge
rem rising, the night.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_0 --child --preset deku-tree-dead --time 17:00 --script dusk ^
    --trace out\run\dusk.json --screenshot out\run\dusk.png --shots-at 150,300,350 %*
exit /b %errorlevel%
