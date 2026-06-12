@echo off
setlocal

set "ROOT=%~dp0.."
set "COMMAND=%~1"
if "%COMMAND%"=="" set "COMMAND=help"

if /I "%COMMAND%"=="help" goto :help
if /I "%COMMAND%"=="status" goto :runtime
if /I "%COMMAND%"=="start" goto :runtime
if /I "%COMMAND%"=="start-local" goto :runtime
if /I "%COMMAND%"=="start-cloud" goto :runtime
if /I "%COMMAND%"=="start-distributed" goto :runtime
if /I "%COMMAND%"=="restart" goto :runtime
if /I "%COMMAND%"=="restart-local" goto :runtime
if /I "%COMMAND%"=="restart-cloud" goto :runtime
if /I "%COMMAND%"=="restart-distributed" goto :runtime
if /I "%COMMAND%"=="stop" goto :runtime
if /I "%COMMAND%"=="export-db" goto :runtime
if /I "%COMMAND%"=="hot-status" goto :runtime
if /I "%COMMAND%"=="hot-start-local" goto :runtime
if /I "%COMMAND%"=="hot-start-cloud" goto :runtime
if /I "%COMMAND%"=="hot-start-distributed" goto :runtime
if /I "%COMMAND%"=="hot-stop" goto :runtime

if /I "%COMMAND%"=="doctor" goto :installer
if /I "%COMMAND%"=="validate-env" goto :installer
if /I "%COMMAND%"=="package" goto :stage_release
if /I "%COMMAND%"=="package-runtime" goto :stage_release
if /I "%COMMAND%"=="install" goto :installer_alias

echo Command "%COMMAND%" is not yet available through scripts\kyuubiki.cmd.
echo Supported commands on Windows today:
echo   status, start*, restart*, stop, export-db
echo   hot-status, hot-start-*, hot-stop
echo   doctor, validate-env, install, package, package-runtime
echo.
echo For advanced desktop build and dev commands, use the Unix entrypoint or extend the Windows wrapper.
exit /b 1

:runtime
shift
cd /d "%ROOT%"
node .\scripts\kyuubiki-runtime.mjs "%COMMAND%" %*
exit /b %ERRORLEVEL%

:installer
shift
cd /d "%ROOT%\workers\rust"
cargo run -p kyuubiki-installer -- "%COMMAND%" %*
exit /b %ERRORLEVEL%

:installer_alias
shift
cd /d "%ROOT%\workers\rust"
cargo run -p kyuubiki-installer -- %*
exit /b %ERRORLEVEL%

:stage_release
shift
cd /d "%ROOT%\workers\rust"
cargo run -p kyuubiki-installer -- stage-release windows %*
exit /b %ERRORLEVEL%

:help
echo kyuubiki Windows entrypoint
echo.
echo Examples:
echo   scripts\kyuubiki.cmd status
echo   scripts\kyuubiki.cmd restart-local
echo   scripts\kyuubiki.cmd doctor
echo   scripts\kyuubiki.cmd package-runtime
exit /b 0
