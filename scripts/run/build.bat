@echo off
rem Builds the whole workspace (release) into the session's target folder.
rem Extra arguments go to cargo, e.g.: build.bat -p oot_sandbox
setlocal
call "%~dp0_env.bat"
cargo build --release --workspace %*
exit /b %errorlevel%
