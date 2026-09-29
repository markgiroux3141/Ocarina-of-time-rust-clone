@echo off
rem Headless: the scripted run from Link's bed on a new save to the Kokiri Sword's chest. Writes
rem its trace and screenshots to out\run\: at the crawlspace's mouth, crawling, out in the
rem training area, waiting for the boulder in the alcove, and the sword held up with its text.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LINK_HOME_0 --child --script sword-chest ^
    --trace out\run\sword_chest.json --screenshot out\run\sword_chest.png --shots-at 975,1080,1205,1330,1700 %*
exit /b %errorlevel%
