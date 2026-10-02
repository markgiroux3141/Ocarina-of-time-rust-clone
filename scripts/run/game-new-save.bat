@echo off
rem The game from Link's bed on a new save (no preset: no sword, no shield, the Deku Tree's mouth
rem shut): GAME-03 milestones 2 to 4's route by hand. Out of the house and down the ladder; west
rem through the village and up the ramp to the plateau; north to the crawlspace by its sign, where
rem A says Enter at the mouth; W to crawl (S backs out); in the training area, dodge the boulder;
rem up to the chest and A. Then Enter (Start, the pause menu's stand-in) puts the sword on B.
rem On to the shop (the Deku Shield is 40 rupees; the run finds 42: room 2's two blue rupees, the
rem plateau sign's hidden switch (slash beside it), four greens, Mido's house's chests, the free
rem multitag's two tag points by the stream, the shop's own right of the counter), Enter again
rem for the shield, and east over the ford to Mido. Past him, along the path through the passage
rem into the Deku Tree's meadow: his talk starts by itself (cutscenes: Space through the texts,
rem Space again for yes at his question), his mouth opens, and in you go.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_LINKS_HOUSE_0 %*
exit /b %errorlevel%
