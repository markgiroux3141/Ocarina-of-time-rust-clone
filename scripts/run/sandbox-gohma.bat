@echo off
rem Headless: GAME-05 milestone 6a's scripted run in Queen Gohma's room, from its entrance's spawn
rem (deku-tree-gohma): Link walks in and her intro plays; he looks up at her with the slingshot until
rem she notices, and she drops with her title card; a seed into her red eye stuns her and jump
rem slashes hurt her; on the ceiling a seed knocks her down; at no health her death plays (her
rem textures erased, the pieces, the blue flashes), and he takes the heart container. Writes its
rem trace and screenshots to out\run\: the title card, the stun, her decay, the heart.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_BOSS_0 --child --preset deku-tree-gohma --script gohma ^
    --trace out\run\gohma.json --screenshot out\run\gohma.png --shots-at 560,700,1430,1700 %*
exit /b %errorlevel%
