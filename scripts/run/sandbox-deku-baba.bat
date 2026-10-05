@echo off
rem Headless: GAME-05 milestone 2's scripted run inside the Deku Tree: a Deku Baba bites Link,
rem then he slashes it while it's stuck, cuts its stem and it's a Deku Stick. Writes its trace
rem and screenshots to out\run\: the bite, the missed bite, stretched out, and the stick.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-inside --script deku-baba ^
    --trace out\run\deku_baba.json --screenshot out\run\deku_baba.png --shots-at 30,110,150,175 %*
exit /b %errorlevel%
