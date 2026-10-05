@echo off
rem Inside the Deku Tree (Master Quest) on the ground floor of its first room, by a withered Deku
rem Baba and under a Keese perched on the wall (the deku-tree-inside preset: the Kokiri Sword and
rem the Deku Shield worn). Walk up to the Deku Baba and it springs up: Q (Z) locks on (the battle
rem camera), E (B) slashes it while it's up, and it leaves a Deku Stick (Space picks it up). The
rem Keese dives at you: lock on and hold R to raise the shield and block it, then slash it while
rem it hovers. R alone (not locked on) is the guard: the stick tilts the shield. A Deku Scrub's
rem nut isn't in this room (the nut bounces back off the Deku Shield in the tests).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-inside --at=-54,0,-247,32768
exit /b %errorlevel%
