@echo off
rem Headless: the playthrough (Link's bed into the Deku Tree). Writes its trace and
rem screenshots to out\run\, one before each step's end: the sign, the Kokiri child, the bush,
rem the tree. (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LINK_HOME_0 --child --preset deku-tree-open --script playthrough ^
    --trace out\run\playthrough.json --screenshot out\run\playthrough.png --shots-at 298,831,994,1878 %*
exit /b %errorlevel%
