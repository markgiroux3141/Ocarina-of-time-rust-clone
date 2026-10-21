@echo off
rem Headless: GAME-10 milestone 1's scripted run, Route::Bottle (Zora's Domain's shallows,
rem zora-bottle): the milk drunk twice, the empty bottle out, a fish caught with a swing, let out.
rem Writes the trace and screenshots to out\run\.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_ZORAS_DOMAIN_0 --child --preset zora-bottle --script bottle ^
    --trace out\run\bottle.json --screenshot out\run\bottle.png --shots-at 30,90,180,300,450,520,560,620,660 %*
exit /b %errorlevel%
