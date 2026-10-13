@echo off
rem Headless: GAME-07 milestone 2's scripted runs. Route::Castle from the castle's road with the
rem Cucco: past the sentries to Talon, the Cucco shown, his run home, two crates pushed into the
rem moat, up them and through the drain into the guards' courtyard (screenshots: Talon, a crate in
rem the moat, on the crates, the courtyard's wave circle, the end). Route::CastleCaught with the
rem egg: into a sentry's sight, the starburst circle and the throw-out by the gate (screenshots:
rem shutting, opening, the end). Writes the traces and screenshots to out\run\.
rem (The frames move when the runs change: the traces' "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_CASTLE_0 --child --preset castle-cucco --script castle ^
    --trace out\run\castle.json --screenshot out\run\castle.png --shots-at 3260,4371,4990,5127 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_CASTLE_0 --child --preset castle-egg --script castle-caught ^
    --trace out\run\castle_caught.json --screenshot out\run\castle_caught.png --shots-at 1434,1460 %*
exit /b %errorlevel%
