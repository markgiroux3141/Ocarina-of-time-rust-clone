@echo off
rem The game from Link's bed on a new save (no preset: no sword, no shield, the Deku Tree's mouth
rem shut): GAME-03 milestone 2's route by hand. Out of the house and down the ladder; west through
rem the village and up the ramp to the plateau; north to the crawlspace by its sign, where A says
rem Enter at the mouth; W to crawl (S backs out); in the training area, dodge the boulder; up to
rem the chest and A. Then Enter (Start, the pause menu's stand-in) puts the sword on B.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_LINK_HOME_0 %*
exit /b %errorlevel%
