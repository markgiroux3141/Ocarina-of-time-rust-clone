@echo off
rem The Lost Woods from Kokiri Forest with Zelda's Letter and the Sun's Song (graveyard-sun): the
rem right turns (east, north, east, north, north, west, then north out of room 8; a wrong turn ends
rem back in Kokiri Forest), the owl in the second room, Saria's Song louder the right way. The
rem meadow: the Wolfos by the entrance rises when you come near (Z, R to guard, B when it's open);
rem its death sinks the gate. The maze's Mad Scrubs spit nuts: R facing one bounces its nut back.
rem Up the ramp, past z -2220, Saria's lesson: play the song back as the staff shows it.
rem "meadow": the meadow's entrance. "saria": on Saria's plateau (the lesson starts at once).
rem "kokiri": a new save in Link's house: Saria's greeting as he walks out.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R (the shield), J C-Left, K C-Down (the
rem ocarina). Saria's Song: K L J K L J.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "entr=ENTR_LOST_WOODS_0"
set "preset=--preset graveyard-sun"
if /i "%~1"=="meadow" (
    set "entr=ENTR_SACRED_FOREST_MEADOW_0"
    shift
)
if /i "%~1"=="saria" (
    set "entr=ENTR_SACRED_FOREST_MEADOW_2"
    shift
)
if /i "%~1"=="kokiri" (
    set "entr=ENTR_LINKS_HOUSE_0"
    set "preset="
    shift
)
"%BIN%\oot.exe" --entrance %entr% %preset% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
