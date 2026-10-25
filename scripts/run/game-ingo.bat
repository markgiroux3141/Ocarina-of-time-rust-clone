@echo off
rem GAME-12 milestone 2, Ingo, the rental and the races, for the hand test.
rem Empty: Lon Lon Ranch by day, an adult before Epona (adult-awake), by Ingo at the corral's gate:
rem talk to him (A): "Ride", listen or not, 10 Rupees: the corral, a minute on the timer. Play
rem Epona's Song (L, then I J L I J L), ride her (or the rental horse); let the minute run out:
rem Ingo stops you (0x2025) and takes you back to the gate.
rem "race": the same past a first ridden rental (ingo-rental, 60 Rupees): rent again, call Epona,
rem ride up to Ingo and talk to him from horseback (Z on him, A): "Do it": the race's intro and the
rem race, one lap round the corral against Ingo (A spurs). Win or lose: Ingo's lines after.
rem "lost": the race just lost (ingo-race-lost): Ingo gloats, takes 50 Rupees, offers a rematch.
rem "won": the first race just won (ingo-first-win): Ingo calls the second race; win it too for
rem Epona (the gate's shutting is milestone 3's).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--adult --entrance ENTR_LON_LON_RANCH_0 --preset adult-awake --at=-40,0,-1700,-8848"
if /i "%~1"=="race" (
    set "args=--adult --entrance ENTR_LON_LON_RANCH_0 --preset ingo-rental --at=-40,0,-1700,-8848"
    shift
) else if /i "%~1"=="lost" (
    set "args=--adult --entrance ENTR_LON_LON_RANCH_8 --preset ingo-race-lost"
    shift
) else if /i "%~1"=="won" (
    set "args=--adult --entrance ENTR_LON_LON_RANCH_7 --preset ingo-first-win"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
