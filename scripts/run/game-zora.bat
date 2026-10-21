@echo off
rem GAME-10's Zora places (milestones 2 and 3, and Player's boomerang) as a child, for the hand
rem tests a script can't do well. Empty: the diving game (Zora's Domain, 50 rupees: talk to the
rem Zora at the top of the falls, 20 rupees a game; dive for the five rupees before the timer
rem runs out; a win gives the silver scale). "king": King Zora with Ruto's letter on C-Left (talk,
rem show it with C-Left when he asks: he mweeps aside). "jabu": Zora's Fountain with a fish on
rem C-Left (walk up to Lord Jabu-Jabu, let the fish out by him: he swallows Link). "lake": Lake
rem Hylia with the silver scale (dive for Ruto's letter by the lab: A held at the surface).
rem "lab": the lakeside lab (dive in the tank with the silver scale: the professor counts).
rem "river": Zora's River (the Octoroks; Zelda's Lullaby on the waterfall's platform opens it;
rem the bean seller by the fence at the river's start sells magic beans). "shop": the Zora shop
rem (talk to the shopkeeper, browse the shelves with the stick, buy with A).
rem "boomerang": Zora's Domain with the boomerang on C-Left (tap to throw, hold to aim, Z-target
rem to send it at something; it brings drops back).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_ZORAS_DOMAIN_0 --preset zora-diving"
if /i "%~1"=="shop" (
    set "args=--entrance ENTR_ZORA_SHOP_0 --preset zora-diving"
    shift
) else if /i "%~1"=="king" (
    set "args=--entrance ENTR_ZORAS_DOMAIN_1 --preset zora-letter"
    shift
) else if /i "%~1"=="jabu" (
    set "args=--entrance ENTR_ZORAS_FOUNTAIN_2 --preset jabu-fish"
    shift
) else if /i "%~1"=="lake" (
    set "args=--entrance ENTR_LAKE_HYLIA_0 --preset lake-dive"
    shift
) else if /i "%~1"=="lab" (
    set "args=--entrance ENTR_LAKESIDE_LABORATORY_0 --preset lake-dive"
    shift
) else if /i "%~1"=="river" (
    set "args=--entrance ENTR_ZORAS_RIVER_0 --preset fairy-magic"
    shift
) else if /i "%~1"=="boomerang" (
    set "args=--entrance ENTR_ZORAS_DOMAIN_0 --preset boomerang --at 560,-20,-420,16384"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
