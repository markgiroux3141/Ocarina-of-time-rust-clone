@echo off
rem Hyrule Field at 18:30 (ENTR_HYRULE_FIELD_0) with the Fairy Ocarina on C-Left and six songs
rem (ocarina-songs: Zelda's Lullaby, Epona's Song, Saria's Song, the Sun's Song, the Song of Time,
rem the Song of Storms), 300 north of the fairy spot by the river (Shot_Sun, a hollow to the south).
rem "day": at 10:00. "night": at 22:00. "fairy": on the fairy spot's rim, within its 150.
rem WASD the stick (Shift to walk), Space A, E B, Q Z, R R, J C-Left (the ocarina out).
rem The ocarina's notes: Space A, K C-Down, L C-Right, J C-Left, I C-Up; R a semitone up, Q down.
rem Saria's Song K L J K L J; the Sun's Song L K I L K I; the Song of Storms Space K I Space K I;
rem Epona's Song I J L I J L; Zelda's Lullaby J I L J I L; the Song of Time L Space K L Space K.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "clock=18:30"
set "at=--at=5143,-30,4700,0"
if /i "%~1"=="day" (
    set "clock=10:00"
    shift
)
if /i "%~1"=="night" (
    set "clock=22:00"
    shift
)
if /i "%~1"=="fairy" (
    set "at=--at=5143,-60,4932,0"
    shift
)
"%BIN%\oot.exe" --entrance ENTR_HYRULE_FIELD_0 --preset ocarina-songs --time %clock% %at% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
