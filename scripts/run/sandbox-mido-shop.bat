@echo off
rem Headless: the scripted run from Link's bed on a new save to the sword, 42 rupees, the Deku
rem Shield from the Kokiri shop, both worn, and past Mido. Writes its trace and screenshots to
rem out\run\: the plateau sign's switch slashed, a chest in Mido's house, browsing the right
rem shelf, the buy prompt, the shield held up, talking to Mido, Mido stepping aside, past him.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LINKS_HOUSE_0 --child --script mido-shop ^
    --trace out\run\mido_shop.json --screenshot out\run\mido_shop.png --shots-at 2850,3450,4509,4525,4640,5200,5650,5731 %*
exit /b %errorlevel%
