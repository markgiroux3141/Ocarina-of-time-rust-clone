@echo off
rem Headless: GAME-05 milestone 5c's save run inside the Deku Tree: the pause menu opened, the
rem slingshot onto C-Left, B's save prompt, Yes (the save written to file 2 of an SRAM in memory,
rem the menu closed), then the console's reset loading file 2 back. Writes its trace, the
rem screenshots (the prompt halfway in, the prompt waiting on Yes, the file loaded) and the final
rem SRAM image (save.sra: ootx sram --path out\run\save.sra lists it) to out\run\.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
if exist out\run\save.sra del out\run\save.sra
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-save --script save ^
    --trace out\run\save.json --screenshot out\run\save.png --shots-at 69,73 --sram out\run\save.sra %*
exit /b %errorlevel%
