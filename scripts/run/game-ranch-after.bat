@echo off
rem GAME-12 milestone 4, the ranch after Epona and Hyrule Field's Poes, for the hand test.
rem Empty: Lon Lon Ranch by day with Epona yours (epona-won), by Malon at the corral's gate. Talk to
rem her (her thanks), then again: "play Epona's Song" until Epona's near; with her near, the
rem obstacle course ("Let's go"). Ride two laps over the eight fences in order within the time;
rem Malon gives your time (under 50 seconds the first time: the record, and a cow at your house).
rem "record": the course just ridden in 45 seconds (malon-record): Malon's record, the cow's unlock.
rem "night": the ranch at 22:00 before Epona (adult-awake), by Malon singing: take out the ocarina
rem (L) by her and play Epona's Song (I J L I J L): she asks if you know her mother's song.
rem "stable": the stable by day before Epona (adult-awake): Malon with her basket.
rem "field": Hyrule Field below the ranch's ramp with Epona yours (epona-won): call her (Epona's
rem Song), ride west towards the ramp's foot: a Big Poe appears ahead and flees. Chase it, hit it
rem with the sword from horseback (B) until it falls, get off and touch its soul: "Yes" puts it in
rem the empty bottle (0x508F). Ride there again: no Big Poe there any more.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--adult --entrance ENTR_LON_LON_RANCH_0 --preset epona-won --at=-40,0,-1700,-8848"
if /i "%~1"=="record" (
    set "args=--adult --entrance ENTR_LON_LON_RANCH_7 --preset malon-record"
    shift
) else if /i "%~1"=="night" (
    set "args=--adult --entrance ENTR_LON_LON_RANCH_0 --preset adult-awake --time 22:00 --at=66,0,-362,-32768"
    shift
) else if /i "%~1"=="stable" (
    set "args=--adult --entrance ENTR_STABLE_0 --preset adult-awake"
    shift
) else if /i "%~1"=="field" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_6 --preset epona-won --at=-1500,61,5600,-6656"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
