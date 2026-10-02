@echo off
rem Headless: the scripted run from Link's bed on a new save past Mido into the Deku Tree, the
rem game's way (no preset). Writes its trace and screenshots to out\run\: walked in by the Deku
rem Tree's first cutscene, his first text, his question, his mouth opening after yes, the talk
rem over, the open jaw, the Deku Tree's intro, and standing inside.
rem (The frames move when the run changes: the trace's "step" and "cutscene" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LINKS_HOUSE_0 --child --script new-save-deku-tree ^
    --trace out\run\new_save_deku_tree.json --screenshot out\run\new_save_deku_tree.png --shots-at 6030,6060,6790,6900,7056,7328,7420,7576 %*
exit /b %errorlevel%
