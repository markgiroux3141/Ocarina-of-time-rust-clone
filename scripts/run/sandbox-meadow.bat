@echo off
rem Headless: GAME-08 milestone 1's scripted run. Route::Meadow from the Lost Woods' entrance with
rem Zelda's Letter and the Sun's Song (graveyard-sun): the owl, the woods' right turns, the meadow,
rem the Wolfos killed, three Mad Scrubs bounced and killed, the maze, the ramp, Saria's lesson and
rem the song played back (screenshots along the way). Writes the trace and screenshots to out\run\.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LOST_WOODS_0 --child --preset graveyard-sun --script meadow ^
    --trace out\run\meadow.json --screenshot out\run\meadow.png --shots-at 900,1830,2600,3850,5300,6300 %*
exit /b %errorlevel%
