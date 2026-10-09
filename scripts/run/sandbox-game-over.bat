@echo off
rem Headless: GAME-05 milestone 5b-2's game over run inside the Deku Tree with a quarter heart: the
rem Deku Baba's bite kills Link, "GAME OVER" fades in, the window turns in, No at "Would you like
rem to save?", Yes at "Continue playing?", and Link starts again at the entrance. Writes its trace
rem and screenshots to out\run\: the message fading in, drawn, the save prompt, the continue
rem prompt, the respawn.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-quarter-heart --script game-over ^
    --trace out\run\game_over.json --screenshot out\run\game_over.png --shots-at 125,170,191,193 %*
exit /b %errorlevel%
