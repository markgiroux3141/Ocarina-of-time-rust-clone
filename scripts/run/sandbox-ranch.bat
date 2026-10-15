@echo off
rem Headless: GAME-08 milestone 3's scripted run, step (a). Route::Ranch from Lon Lon Ranch's gate
rem with the Lost Woods' games won (lost-woods-games): the intro, Malon's three talks and her
rem lesson (Epona's Song played back), the song's scene and back (destination 94), the foal coming
rem to Link, and called again with the song. Writes the trace and screenshots to out\run\.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_0 --child --preset lost-woods-games --script ranch ^
    --trace out\run\ranch.json --screenshot out\run\ranch.png --shots-at 600,880,1600,2000,2290,3000 %*
exit /b %errorlevel%
