@echo off
rem Dodongo's Cavern's entrance with the Goron's Bracelet and the 20 bomb bag (cavern-bombs, a
rem debug start). The Bomb Flowers either side on the raised strips: A by one to lift it (without
rem the bracelet Link only tugs), carry it to the wall ahead and put it down standing still (A),
rem then get away: the wall breaks and the cavern's opening plays. A bomb: C-Right (L) takes one
rem out; A or L again puts it down (or throws it while moving). Its fuse runs about 2 seconds.
rem "field": Hyrule Field by day with bombs: a Peahat's death explodes; throw bombs about.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down (the ocarina),
rem L C-Right (the bombs), I C-Up.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_DODONGOS_CAVERN_0 --preset cavern-bombs"
if /i "%~1"=="field" (
    set "args=--entrance ENTR_HYRULE_FIELD_0 --preset cavern-bombs --time 10:00"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
