@echo off
rem Headless: GAME-07 milestone 1's scripted run (Route::Market): from Castle Town's entrance by
rem day into the market, the red-shirted townsman's talk, Malon's, and north out to the castle's
rem road, where its intro plays. Writes the trace and screenshots to out\run\ (the townsman's
rem text, Malon's, the castle's intro, the end).
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_MARKET_ENTRANCE_DAY_0 --child --preset deku-tree-dead --script market ^
    --trace out\run\market.json --screenshot out\run\market.png --shots-at 380,860,1120 %*
exit /b %errorlevel%
