@echo off
rem Headless: GAME-11 milestone 3's scripted runs. prelude (prelude, a debug start: Sheik's
rem Prelude of Light), sword-back (prelude-learned: the sword put back, a child), sword-again
rem (child-again: pulled again, an adult) and prelude-warp (prelude-learned, the field: the warp to
rem the Temple's light pad). (--child: the save the presets start from; they make Link an adult.)
rem Writes the traces and screenshots to out\run\. (The frames move when a run changes: the
rem trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_0 --child --preset prelude --script prelude ^
    --trace out\run\prelude.json --screenshot out\run\prelude.png --shots-at 400,700,1000,1300,1700,2100,2300 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_0 --child --preset prelude-learned --script sword-back ^
    --trace out\run\sword-back.json --screenshot out\run\sword-back.png --shots-at 60,200,400 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_TEMPLE_OF_TIME_2 --child --preset child-again --script sword-again ^
    --trace out\run\sword-again.json --screenshot out\run\sword-again.png --shots-at 60,200,400 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_7 --child --preset prelude-learned --script prelude-warp ^
    --trace out\run\prelude-warp.json --screenshot out\run\prelude-warp.png --shots-at 70,200,240,280,330,380 %*
exit /b %errorlevel%
