@echo off
rem GAME-04b milestone 7's tests: the cutscenes against the C frame by frame. The narration's and
rem the Deku Tree's talk's camera on their splines (Camera_Demo1), the letterbox growing and
rem shrinking by 10 rows, the narration's text box placement, Link's pose on each opening cue.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test polish %*
exit /b %errorlevel%
