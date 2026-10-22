@echo off
rem Headless: GAME-10 milestone 7's two scripted runs. Route::Escape (Hyrule Field short of the
rem drawbridge, zora-sapphire): the bridge's trigger and Zelda's escape. Route::OcarinaOfTime (the
rem moat's bank, zelda-fled): the dive for the Ocarina of Time, the Song of Time's cutscenes through
rem the Temple of Time, the field after.
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_0 --child --preset zora-sapphire --script zelda-escape ^
    --trace out\run\zelda_escape.json --screenshot out\run\zelda_escape.png --shots-at 30,100,400,800,1000,1300,1500,1800,2200,2600,2900 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_0 --child --preset zelda-fled --script ocarina-of-time ^
    --trace out\run\ocarina_of_time.json --screenshot out\run\ocarina_of_time.png --shots-at 40,250,311,400,470,600,900,1300,1488,1800,2100,2187 %*
exit /b %errorlevel%
