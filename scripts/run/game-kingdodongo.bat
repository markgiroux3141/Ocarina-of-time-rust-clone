@echo off
rem King Dodongo's room as a child after milestone 3 (cavern-boss: the bombs on C-Right). Empty: the
rem room's spawn on the ledge: a bomb on the cracked floor, the drop, his intro, the fight (the hand
rem test). "fight": the arena beside him, his battle begun (cavern-boss-fight: a second try's short
rem intro, then the fight). "beaten": the arena's east rim after his defeat (cavern-boss-beaten: the
rem heart container, the blue warp). "ruby": the Goron's Ruby's cutscene on Death Mountain Trail
rem (goron-ruby, cutscene 0xFFF1). "trail": the trail's spawn 5 with the Ruby (goron-ruby: Phase 10's
rem exit, milestones 5 and 6's start).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_DODONGOS_CAVERN_BOSS_0 --preset cavern-boss"
if /i "%~1"=="fight" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_BOSS_0 --preset cavern-boss-fight --at -890,-1504,-2804,-16384"
    shift
) else if /i "%~1"=="beaten" (
    set "args=--entrance ENTR_DODONGOS_CAVERN_BOSS_0 --preset cavern-boss-beaten --at -470,-1504,-3304,-16384"
    shift
) else if /i "%~1"=="ruby" (
    set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_0 --cutscene 0xFFF1 --preset goron-ruby"
    shift
) else if /i "%~1"=="trail" (
    set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_5 --preset goron-ruby"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
