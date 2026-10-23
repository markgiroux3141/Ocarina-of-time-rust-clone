@echo off
rem GAME-10 milestone 9's frogs and scarecrows, for the hand test (the ocarina on C-Down, all six
rem of a child's songs, the Song of Storms a debug start's). Empty: Zora's River on the frogs'
rem log (take the ocarina out there: Link is put on the spot and the camera turns to the frogs;
rem play a single note: the frog of its colour jumps; play Zelda's Lullaby, Epona's Song, Saria's
rem Song, the Sun's Song, the Song of Time: the frogs dance, one grows and gives a purple rupee the
rem first time each; the Song of Storms: a piece of heart). "choir": the same with the six songs
rem already played (take the ocarina out on the log: the frogs' song game; play each note as the
rem butterfly lands on its frog's colour, B gives up; all fourteen: a piece of heart).
rem "lake": Lake Hylia in front of Bonooru, one of the two scarecrows by the lab (take the ocarina
rem out: he asks for a song; play eight notes that aren't one of your songs: he remembers it; then
rem talk to him: he plays it back). "longsong": in front of the other scarecrow (take the
rem ocarina out: he records as long as you play, B ends it; then talk to him: he plays it back).
rem "bonooru": adult Lake Hylia in front of Bonooru with a song recorded as a child (A, C-Down, A,
rem C-Up twice over; take the ocarina out: he asks to hear it; play it back: he's glad). "pierre":
rem adult Lake Hylia after that, by Pierre's spot on the lab's shore (play the scarecrow's song:
rem Pierre rises from the ground).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_ZORAS_RIVER_0 --preset ocarina-c --at 990,205,-1220,5461"
if /i "%~1"=="choir" (
    set "args=--entrance ENTR_ZORAS_RIVER_0 --preset frogs-choir --at 990,205,-1220,5461"
    shift
) else if /i "%~1"=="lake" (
    set "args=--entrance ENTR_LAKE_HYLIA_0 --preset ocarina-c --at -154,-1293,2958,-32768"
    shift
) else if /i "%~1"=="longsong" (
    set "args=--entrance ENTR_LAKE_HYLIA_0 --preset ocarina-c --at 12,-1253,2740,-32768"
    shift
) else if /i "%~1"=="pierre" (
    set "args=--adult --entrance ENTR_LAKE_HYLIA_0 --preset scarecrow-pierre --at 1337,-1269,3373,-16384"
    shift
) else if /i "%~1"=="bonooru" (
    set "args=--adult --entrance ENTR_LAKE_HYLIA_0 --preset scarecrow-song --at -164,-1293,2995,-32768"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
