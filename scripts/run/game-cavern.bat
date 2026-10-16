@echo off
rem Dodongo's Cavern (Master Quest) as a child after milestone 2 (cavern-open: the bracelet, no
rem bombs). Empty: the cavern's entrance, the whole loop to the skull's bridge and the boss's
rem door (the hand test). "lizalfos": room 1's corridor by door 9, the lower Lizalfos pair beyond
rem it. "pillar": the lobby's island by its Bomb Flower (cavern-lobby: the entrance wall
rem broken), the pit's boulder and the shortcut pillar. "upper": the upper level by room 9's door
rem (cavern-upper: the pillar run's end). "web": room 10 past its web (cavern-web: the web burned,
rem no way found for a child to bring fire to it). "bag": the lobby's east corridor by the
rem bomb bag's chest (cavern-loop: the loop done). "skull": the bridge over the skull with the
rem bag (cavern-bomb-bag). "gravestone": room 7's alcove north of the gravestone, the jaw open
rem (cavern-jaw). "boss": King Dodongo's room (cavern-boss: milestone 4's start).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-open"
if /i "%~1"=="lizalfos" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-lobby --room 1 --at 3560,100,-1120,-32768"
    shift
) else if /i "%~1"=="pillar" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-lobby --at -2,0,-1240,0"
    shift
) else if /i "%~1"=="upper" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-upper --at 1060,531,-320,16384"
    shift
) else if /i "%~1"=="web" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-web --room 10 --at 2660,531,-320,16384"
    shift
) else if /i "%~1"=="bag" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-loop --at 1000,80,-920,-16384"
    shift
) else if /i "%~1"=="skull" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-bomb-bag --at 0,591,-1535,16384"
    shift
) else if /i "%~1"=="gravestone" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_1 --preset cavern-jaw --at 0,126,-3300,0"
    shift
) else if /i "%~1"=="boss" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_BOSS_0 --preset cavern-boss"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
