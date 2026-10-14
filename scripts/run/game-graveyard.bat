@echo off
rem The graveyard at 18:30 with Zelda's Lullaby (graveyard-lullaby: the Fairy Ocarina on C-Down): its
rem intro, then the graves. Dampe (south of the first row) digs for 10 rupees. The royal tomb is
rem east, up the ramps round the south end of the wall between the rows: on its terrace (rain and
rem thunder there) play the lullaby on the spot in front of the big gravestone. Inside: the Keese,
rem the door, the ReDeads (keep out of their sight), and the wall with the Sun's Song.
rem "day": 10:00 by Dampe's hut (the graveyard kid on his round). "dampe": 18:30 by Dampe's hut.
rem "tomb": 18:30 on the tomb's terrace. "inside": the royal tomb from its entrance.
rem "redeads": by the tomb's wall with the Sun's Song learned (graveyard-sun): back into the ReDeads'
rem room and play it at them. "redead": the ReDead grave at 18:30 (graveyard-sun). "shield": the
rem shield grave (its wall stands: there are no bombs yet). "hut": Dampe's hut.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R (the shield), J C-Left, K C-Down (the
rem ocarina). Zelda's Lullaby: J I L J I L. The Sun's Song: L K I L K I.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "entr=ENTR_GRAVEYARD_0"
set "preset=graveyard-lullaby"
set "clock=18:30"
set "at="
if /i "%~1"=="day" (
    set "entr=ENTR_GRAVEYARD_2"
    set "clock=10:00"
    shift
)
if /i "%~1"=="dampe" (
    set "entr=ENTR_GRAVEYARD_2"
    shift
)
if /i "%~1"=="tomb" (
    set "entr=ENTR_GRAVEYARD_6"
    shift
)
if /i "%~1"=="inside" (
    set "entr=ENTR_ROYAL_FAMILYS_TOMB_0"
    shift
)
if /i "%~1"=="redeads" (
    set "entr=ENTR_ROYAL_FAMILYS_TOMB_1"
    set "preset=graveyard-sun"
    shift
)
if /i "%~1"=="redead" (
    set "entr=ENTR_REDEAD_GRAVE_0"
    set "preset=graveyard-sun"
    shift
)
if /i "%~1"=="shield" (
    set "entr=ENTR_GRAVE_WITH_FAIRYS_FOUNTAIN_0"
    shift
)
if /i "%~1"=="hut" (
    set "entr=ENTR_GRAVEKEEPERS_HUT_0"
    shift
)
"%BIN%\oot.exe" --entrance %entr% --preset %preset% --time %clock% %at% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
