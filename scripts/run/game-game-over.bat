@echo off
rem Inside the Deku Tree (Master Quest) on its top floor with a quarter heart left, 85 from a
rem Deku Baba and facing it (the deku-tree-quarter-heart preset). Stand still: it bites and Link
rem dies. The game over follows: "GAME OVER" fading in over the scene, then the window turning in
rem with "Would you like to save?": D (the stick right) and Space (A) for No, or Space for Yes;
rem then "Continue playing?": Space for Yes, and Link starts again at the Deku Tree's entrance
rem with three hearts. (Yes at the save prompt saves to an SRAM in memory: a debug start's file
rem isn't your save file. No at "Continue playing?" would go to the title screen, which isn't
rem ported: it continues.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_0 --preset deku-tree-quarter-heart --at=-255.104,800,-255.104,8192 %*
exit /b %errorlevel%
