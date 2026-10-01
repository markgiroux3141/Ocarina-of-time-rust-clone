@echo off
rem A new file as the file select starts it (GAME-03 milestone 5): the opening plays, then you
rem play. The Deku Tree's narration over Link asleep in his house, his nightmare (Hyrule Field
rem at night, Zelda's escape: its actors are placeholders), the Deku Tree sending Navi, her
rem flight through the village, and her waking Link. Space (A) at the texts that wait. Enter
rem (Start) skips a scene, as in the game.
rem Then Navi is Link's: she follows him, flies to what Q (Z) would target, hides in his cap when
rem nothing is around, and after about 30 s in one place (naviTimer 600) she calls: I (C-Up)
rem talks to her ("The Great Deku Tree wants to talk to you!").
rem The rest is the new save's way: the sword, the shop, Mido, the Deku Tree (game-new-save.bat).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --new-file %*
exit /b %errorlevel%
