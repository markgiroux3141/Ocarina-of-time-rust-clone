@echo off
rem Headless: GAME-07 milestone 5's scripted run. Route::Kakariko from Kakariko's entrance by day with
rem Zelda's Letter (kakariko-letter): its intro, the cucco by the way in carried to Anju's pen and thrown
rem over its fence, the one east of the pen, the one on the landing below the gate's stairs glided down
rem off the low roof to the pen, then up to the gate guard: the letter shown, the gate opened
rem (screenshots: carrying, a throw by the pen, the glide, the guard, the end). Writes the trace and
rem screenshots to out\run\. (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_KAKARIKO_VILLAGE_0 --child --preset kakariko-letter --script kakariko ^
    --trace out\run\kakariko.json --screenshot out\run\kakariko.png --shots-at 600,908,2140,4000 %*
exit /b %errorlevel%
