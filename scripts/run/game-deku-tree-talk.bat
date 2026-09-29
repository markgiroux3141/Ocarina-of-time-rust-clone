@echo off
rem A shortcut to the Deku Tree's first talk: a new save entering his meadow (ENTR_SPOT04_1),
rem facing him. His talk (Bg_Treemouth's cutscene D_808BCE20) starts at once: the camera takes
rem the script's shots, Link walks in by himself, and the tree speaks. Space (A) through the
rem texts; at his question, Space for yes: the next cutscene (D_808BD520) opens his mouth. W/S
rem (the stick) picks no instead (D_808BD790); then Q (Z) targeting the tree asks again.
rem Walk into the open mouth for the Deku Tree, whose intro plays the first time in.
rem The whole way from a new save, the game's way: game-new-save.bat.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_SPOT04_1 %*
exit /b %errorlevel%
