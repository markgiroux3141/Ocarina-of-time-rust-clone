@echo off
rem The render and trace regression: every golden case against golden\renders.sha256.
rem Extra arguments go to golden.py, e.g.: golden-check.bat --only spot04
rem (Recording new hashes is deliberate: python scripts\golden.py record, logged in golden\README.md.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
python scripts\golden.py check --bin-dir "%BIN%" %*
exit /b %errorlevel%
