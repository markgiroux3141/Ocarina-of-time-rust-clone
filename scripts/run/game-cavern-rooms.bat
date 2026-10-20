@echo off
rem Dodongo's Cavern's rest (GAME-09 milestone 5) as a child with magic and bombs (fairy-magic).
rem Empty: room 2 by its south door (the Beamos, the Bomb Flowers, the silver rupees, the bombable
rem wall to room 15). "stairs": room 2 south of the Bomb Flowers' strip (the falling stairs: light
rem the rows at their junction). "rupees": room 2's balcony by the last silver rupee's crate.
rem "beamos": room 2's west side, a Beamos ahead. "dodongos": room 5 inside door 17 (the three
rem Dodongos, the compass's chest). "larvae": room 6 inside door 16 (the Gohma larvae, the Deku
rem Shield's chest). "scrubs": room 13 inside door 14 (the Mad Scrubs, a Keese). "poe": room 14
rem inside door 13 (the Poe, the gravestone over the chest). "blocks": room 15 inside door 15 with
rem the Song of Time (cavern-song-of-time: the time blocks, the Skulltulas, the Gold Skulltula).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 2 --at -1960,0,-1150,-32768"
if /i "%~1"=="stairs" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 2 --at -1959,0,-1180,-32768"
    shift
) else if /i "%~1"=="rupees" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 2 --at -1650,531,-1110,16384"
    shift
) else if /i "%~1"=="beamos" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 2 --at -2277,0,-1150,-32768"
    shift
) else if /i "%~1"=="dodongos" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 5 --at -1960,531,-650,0"
    shift
) else if /i "%~1"=="larvae" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 6 --at 1840,411,-1550,-32768"
    shift
) else if /i "%~1"=="scrubs" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 13 --at 2433,100,-2960,-32768"
    shift
) else if /i "%~1"=="poe" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset fairy-magic --room 14 --at 1473,76,-4300,-32768"
    shift
) else if /i "%~1"=="blocks" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-song-of-time --room 15 --at -1958,0,-650,-32768"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
