@rem Shared settings for the scripts in this folder. Each script calls this after its
@rem setlocal, so nothing leaks into your terminal. Don't run it on its own.
@rem
@rem A session that builds into a new target folder, or imports a new pack format into a new
@rem data folder, changes these two names.
set "TARGET_NAME=game19"
set "DATA_NAME=data17"

@rem The repo's root: two folders up from this file.
for %%I in ("%~dp0..\..") do set "REPO=%%~fI"
set "CARGO_TARGET_DIR=%REPO%\target\%TARGET_NAME%"
@rem The game and the tests find the pack here (oot_game::pack::ENV_DATA_DIR).
set "OOT_DATA_DIR=%REPO%\out\%DATA_NAME%"
set "BIN=%CARGO_TARGET_DIR%\release"
cd /d "%REPO%"
exit /b 0
