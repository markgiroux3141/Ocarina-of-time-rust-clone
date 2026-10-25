@echo off
rem GAME-12 milestone 1, riding Epona, for the hand test.
rem Empty: Lon Lon Ranch's corral by day, an adult before Epona (adult-awake): Epona runs from
rem you. Take out the ocarina (L: C-Right) and play Epona's Song (I J L I J L): she comes. Walk to
rem her side, A ("Climb"): on her. The stick rides (a walk, a trot, a gallop); A spurs her on (a
rem carrot gone, back over time); at a gallop, ride straight at one of the corral's two low
rem fences: she jumps it. Let go of the stick: she stops; A ("Down"): off her.
rem "field": Hyrule Field below the ranch's gate with Epona had (epona): play Epona's Song, on
rem her, up the ramp and through the gate on her: in the ranch still on her, galloping.
rem "ranch": the ranch with Epona had (epona): no rental horse; her song calls her.
rem "night": the ranch at 22:00 before Epona (adult-awake): no horses.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--adult --entrance ENTR_LON_LON_RANCH_0 --preset adult-awake --at=400,0,-800,-9672"
if /i "%~1"=="field" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_6 --preset epona --at=-1500,61,5600,-6656"
    shift
) else if /i "%~1"=="ranch" (
    set "args=--adult --entrance ENTR_LON_LON_RANCH_0 --preset epona"
    shift
) else if /i "%~1"=="night" (
    set "args=--adult --entrance ENTR_LON_LON_RANCH_0 --preset adult-awake --time 22:00"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
