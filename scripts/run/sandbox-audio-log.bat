@echo off
rem Headless: the new file's run (the opening, Navi, the sword, the shop, Mido, the Deku Tree)
rem with the audio library running offline alongside: the sequence commands and the library's
rem commands by frame, and what each player played, into out\run\new_file_deku_tree_audio.json,
rem and the run's sound as out\run\new_file_deku_tree.wav (about 10 minutes); then opens it.
rem The log lists the sound effects asked for. The cutscenes have no music of their own yet
rem (BACKLOG #10), and most actors have no sounds yet.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist "%REPO%\out\run" mkdir "%REPO%\out\run"
"%BIN%\oot_sandbox.exe" --new-file --child --script new-file-deku-tree --audio-log "%REPO%\out\run\new_file_deku_tree_audio.json" --wav "%REPO%\out\run\new_file_deku_tree.wav" %* || exit /b 1
start "" "%REPO%\out\run\new_file_deku_tree.wav"
exit /b 0
