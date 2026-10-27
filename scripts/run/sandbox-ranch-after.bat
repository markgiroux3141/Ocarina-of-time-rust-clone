@echo off
rem Headless: GAME-12 milestone 4's scripted runs. malon-course (epona-won: Malon's thanks, Epona's
rem Song, the course taken, its intro to its start), malon-record (malon-record: the record, the
rem cow's unlock), malon-song (adult-awake at 22:00: Epona's Song to Malon) and big-poe (epona-won
rem in the field: on Epona to the Big Poe's place by the ranch's ramp, it appears and flees).
rem (--child: the save the presets start from; they make Link an adult.) Writes the traces and
rem screenshots to out\run\. (The frames move when a run changes: the trace's "step" fields say
rem where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_0 --child --preset epona-won --script malon-course ^
    --trace out\run\malon-course.json --screenshot out\run\malon-course.png --shots-at 400,1000,1500,1800 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_7 --child --preset malon-record --script malon-record ^
    --trace out\run\malon-record.json --screenshot out\run\malon-record.png --shots-at 100,200 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_LON_LON_RANCH_0 --child --preset adult-awake --time 22:00 --script malon-song ^
    --trace out\run\malon-song.json --screenshot out\run\malon-song.png --shots-at 30,400,550 %* || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_6 --child --preset epona-won --script big-poe ^
    --trace out\run\big-poe.json --screenshot out\run\big-poe.png --shots-at 300,484,500 %*
exit /b %errorlevel%
