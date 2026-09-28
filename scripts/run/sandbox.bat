@echo off
rem The dev sandbox; the arguments go to it. With none: the test course, adult Link.
rem e.g.: sandbox.bat --scene spot04 --child --entrance --placeholders
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
"%BIN%\oot_sandbox.exe" %*
exit /b %errorlevel%
