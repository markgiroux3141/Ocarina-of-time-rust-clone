@echo off
rem Headless: GAME-11 milestone 1's two scripted runs. Route::MasterSwordPull (master-sword, a child
rem on the pedestal's step): the pull, the age change, the adult's arrival on the pedestal.
rem Route::MasterSwordBack (master-sword-adult, the adult arriving on the pedestal): the sword put
rem back, the child's arrival. (--child: the save the preset starts from; the preset makes Link
rem an adult.)
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_0 --child --preset master-sword --script master-sword-pull ^
    --trace out\run\master-sword-pull.json --screenshot out\run\master-sword-pull.png --shots-at 20,90,140,230,290,330,384 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_2 --child --preset master-sword-adult --script master-sword-back ^
    --trace out\run\master-sword-back.json --screenshot out\run\master-sword-back.png --shots-at 20,60,112,180,250,340,380,425 %*
exit /b %errorlevel%
