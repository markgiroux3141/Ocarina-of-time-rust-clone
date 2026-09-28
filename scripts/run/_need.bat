@rem Called with a binary's name (oot, oot_sandbox, ootx): fails with a hint if it isn't built.
@if exist "%BIN%\%~1.exe" exit /b 0
@echo %BIN%\%~1.exe isn't built yet: run scripts\run\build.bat first.
@exit /b 1
