@echo off
rem Runs the tests (release). With no arguments, the whole workspace; otherwise the arguments
rem go to cargo test, e.g.: test.bat -p oot_actors --test talk
setlocal
call "%~dp0_env.bat"
if "%~1"=="" (
    cargo test --release --workspace
) else (
    cargo test --release %*
)
exit /b %errorlevel%
