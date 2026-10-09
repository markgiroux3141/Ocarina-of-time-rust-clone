@echo off
rem A file of your save file (out\saves\<ROM SHA-1>.sra, OOT_SAVE_DIR in _env.bat), as the
rem file select loads it: game-file.bat 2 (the default) or 3. An empty file is made new with the
rem opening: game-file.bat 2 --new-file. File 1 is this debug ROM's map select file: it needs an
rem entrance, e.g. game-file.bat 1 --entrance ENTR_DEKU_TREE_0. Saving (Enter, then E, then
rem Space on Yes) writes the file; F5, the console's reset, loads it again from the disk.
rem ootx sram lists the files (ootx sram erase N, ootx sram copy N M).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "FILE=%~1"
if "%FILE%"=="" set "FILE=2"
"%BIN%\oot.exe" --file %FILE% %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
