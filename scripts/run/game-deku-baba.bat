@echo off
rem Inside the Deku Tree (Master Quest) on its top floor, 85 from a Deku Baba and facing it
rem (the deku-tree-inside preset: the sword and shield worn, the intro seen). Stand still and it
rem rises and bites (half a heart). Q (Z) locks on; E (B) slashes. Hit it while it's stuck to the
rem floor after a missed bite and it lies stretched out: cut its stem for a Deku Stick. Let it
rem bite you down to nothing to die: the game over runs, its menu not drawn (the pause menu's
rem stand-in): when the console says "Save?", D (the stick right) then Space (A) for No, or Space
rem for Yes; at "Continue?" Space again: the screen goes black and Link starts again at the
rem Deku Tree's entrance with three hearts.
rem   game-deku-baba.bat fairy   also gives Link a fairy in a bottle: dying, it revives him.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "PRESET=deku-tree-inside"
if /i "%~1"=="fairy" set "PRESET=deku-tree-inside-fairy"
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset %PRESET% --at=-255.104,800,-255.104,8192
exit /b %errorlevel%
