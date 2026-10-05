@echo off
rem Inside the Deku Tree with the sandbox's training dummy 60 in front of Link, its touch hurting
rem him with a hit kind: none (a plain hit: the stagger), fire (Link burns: the Deku Shield burns
rem away), ice (frozen in a block of ice: mash Space (A) to break out), electric (shocked for
rem twenty frames), knockback (knocked down). Walk into it (W). The red flash shows while Link
rem is invincible after each hit. E (B) slashes the dummy (it flashes red and never dies).
rem   game-dummy.bat ice
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "KIND=%~1"
if "%KIND%"=="" set "KIND=none"
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-inside --at=-4,0,450,-32768 --target 60 --target-hurts %KIND%
exit /b %errorlevel%
