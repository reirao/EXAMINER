@echo off
setlocal
pushd "%~dp0"
set "EXAMINER_EXE=%~dp0bin\examiner.exe"
if not exist "%EXAMINER_EXE%" set "EXAMINER_EXE=%~dp0target\release\examiner.exe"
if not exist "%EXAMINER_EXE%" (
    echo Build EXAMINER first: cargo build --release --offline
    pause
    popd
    exit /b 1
)
"%EXAMINER_EXE%" hook --seconds 3600
pause
popd
