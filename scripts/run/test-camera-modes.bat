@echo off
rem GAME-04b milestone 2's tests: the camera modes that were on the Normal1 fallback, against the
rem C's arithmetic on the pack's camera data: Camera_Jump2 (the ladder: the turn behind Link, the
rem distance, level near the ground), Camera_Jump1 (in the air: the distance and pitch clamped, the
rem eye's height eased), Camera_Unique1 (hanging: the pitch target, the turn to his side). Also
rem the climbing and ledge tests that run through them.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_game --lib camera::tests %* || exit /b 1
cargo test --release -p oot_actors --test climb --test ledge %*
exit /b %errorlevel%
