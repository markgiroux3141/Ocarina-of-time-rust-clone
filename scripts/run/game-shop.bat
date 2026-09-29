@echo off
rem A shortcut to the Kokiri shop: Kokiri Forest outside its door (ENTR_SPOT04_4, where Link
rem comes out), on the sword-and-40-rupees save preset (the Kokiri Sword worn, 40 rupees, no
rem shield). Walk in; Space (A) at the counter talks to the shopkeeper; the stick (WASD) left or
rem right browses a shelf; Space on the Deku Shield, Space on "Buy", Space through its text; E
rem (B) leaves. Enter (Start: the pause menu's stand-in) puts the shield on. Then east over the
rem ford to Mido by the path to the Deku Tree: he steps aside once both are worn.
rem The whole way from a new save, the game's way: game-new-save.bat.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_SPOT04_4 --preset sword-and-40-rupees %*
exit /b %errorlevel%
