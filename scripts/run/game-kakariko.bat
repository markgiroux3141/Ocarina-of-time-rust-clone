@echo off
rem Kakariko by day from its entrance with Zelda's Letter (kakariko-letter: the letter on C-Left, the
rem Fairy Ocarina on C-Down, Zelda's Lullaby): its intro, then the village. Space A lifts a cucco (or a
rem pot, a small rock), A throws it; carried off an edge it glides. Anju's pen is fenced: throw the
rem cuccos over its north fence. The gate guard up the north stairs takes the letter (J, C-Left).
rem "pen": by Anju's pen, its fence ahead. "roof": on the landing below the gate's stairs by its cucco
rem (the low roof south of the watchtower is to glide off). "gate": by the gate guard. "night": the
rem village at 22:00 (the carpenter's son, no cuccos, the windows lit). "dusk": 17:55 by the pen (the
rem windows light at 18:00). "cow": Impa's house's cow pen with the songs (ocarina-songs; Epona's
rem Song). "windmill": the windmill with the songs (the Song of Storms drains the well).
rem "skulltula": the House of Skulltula. "guests": the guest house at 22:00 (the carpenters, Mutoh).
rem "shop": the potion shop (a carpenter). "guardhouse": Castle Town's guard house (44 pots to lift).
rem WASD the stick (Shift to walk), Space A, E B, Q Z, J C-Left, K C-Down (the ocarina).
rem Epona's Song: I J L I J L. The Song of Storms: Space K I Space K I.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "entr=ENTR_KAKARIKO_VILLAGE_0"
set "preset=kakariko-letter"
set "clock=10:00"
set "at="
if /i "%~1"=="pen" (
    set "entr=ENTR_KAKARIKO_VILLAGE_1"
    set "at=--at=330,80,1300,0"
    shift
)
if /i "%~1"=="roof" (
    set "entr=ENTR_KAKARIKO_VILLAGE_1"
    set "at=--at=-60,320,-560,-16384"
    shift
)
if /i "%~1"=="gate" (
    set "entr=ENTR_KAKARIKO_VILLAGE_1"
    set "at=--at=0,400,-1230,-28672"
    shift
)
if /i "%~1"=="night" (
    set "entr=ENTR_KAKARIKO_VILLAGE_1"
    set "clock=22:00"
    set "at=--at=-700,0,500,16384"
    shift
)
if /i "%~1"=="dusk" (
    set "entr=ENTR_KAKARIKO_VILLAGE_1"
    set "clock=17:55"
    set "at=--at=330,80,1300,0"
    shift
)
if /i "%~1"=="cow" (
    set "entr=ENTR_IMPAS_HOUSE_1"
    set "preset=ocarina-songs"
    shift
)
if /i "%~1"=="windmill" (
    set "entr=ENTR_WINDMILL_AND_DAMPES_GRAVE_1"
    set "preset=ocarina-songs"
    shift
)
if /i "%~1"=="skulltula" (
    set "entr=ENTR_HOUSE_OF_SKULLTULA_0"
    shift
)
if /i "%~1"=="guests" (
    set "entr=ENTR_KAKARIKO_CENTER_GUEST_HOUSE_0"
    set "clock=22:00"
    shift
)
if /i "%~1"=="shop" (
    set "entr=ENTR_POTION_SHOP_KAKARIKO_0"
    shift
)
if /i "%~1"=="guardhouse" (
    set "entr=ENTR_MARKET_GUARD_HOUSE_0"
    shift
)
"%BIN%\oot.exe" --entrance %entr% --preset %preset% --time %clock% %at% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
