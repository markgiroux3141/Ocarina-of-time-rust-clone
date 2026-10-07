@echo off
rem GAME-05 milestone 4c's tests: Player's push and pull (holding on to a block, pushing it a
rem block length frame by frame, letting go, pulling it back to its start), room 3's push block
rem (Obj_Oshihiki and Obj_Makeoshihiki: off the channel's end into the pit, flag 0x10 and the
rem chime, where it spawns by its flags, the strength a block needs, a block riding on another),
rem room 7's gravestones (Bg_Haka), the Song of Time's blocks (Obj_Timeblock), room 2's rocks
rem (Obj_Bombiwa), the room-to-room travel test, the debug starts, the exit's run (Route::Push),
rem and the engine's DynaPoly push fields.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test push --test push_run --test travel --test gravestones --test timeblocks --test rocks --test debug_starts %* || exit /b 1
cargo test --release -p eng_collision --lib
exit /b %errorlevel%
