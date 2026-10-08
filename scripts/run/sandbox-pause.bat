@echo off
rem Headless: GAME-05 milestone 5b-1's scripted run inside the Deku Tree: Start opens the pause
rem menu, the item page's cursor goes to the Fairy Slingshot, C-Right equips it (its icon flies to
rem the button), R turns the menu to the map page, Start closes it: the slingshot is on C-Right.
rem Writes its trace and screenshots to out\run\: the menu opening, the cursor on the slingshot,
rem the icon in flight, the map page, the game resumed.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-slingshot-owned --script pause ^
    --trace out\run\pause.json --screenshot out\run\pause.png --shots-at 40,56,60,82,93 %*
exit /b %errorlevel%
