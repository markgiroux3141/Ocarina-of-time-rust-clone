@echo off
rem Headless: GAME-09 milestone 1's scripted run. Route::Bombs from Dodongo's Cavern's spawn with
rem the Goron's Bracelet and the 20 bomb bag (cavern-bombs): the west Bomb Flower lifted and put
rem down at the entrance wall, the blast and the cavern's opening; a bomb out of the bag put down
rem by the east flower, which its blast sets off. Writes the trace and screenshots to out\run\.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DODONGOS_CAVERN_0 --child --preset cavern-bombs --script bombs ^
    --trace out\run\bombs.json --screenshot out\run\bombs.png --shots-at 120,200,250,300,500,520,560 %*
exit /b %errorlevel%
