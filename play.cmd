@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem The project name names the launch-output folder under %LOCALAPPDATA%; keep it equal to the host's data folder name.
set "APP_NAME=Protui"
set "APP_REPO_ROOT=%~dp0"
set "REPO_ID="
for /f "usebackq delims=" %%H in (`powershell.exe -NoLogo -NoProfile -NonInteractive -Command "$p = $env:APP_REPO_ROOT.TrimEnd('\').ToUpperInvariant(); $h = [System.Security.Cryptography.SHA256]::Create(); try { [System.BitConverter]::ToString($h.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($p))).Replace('-', '').ToLowerInvariant() } finally { $h.Dispose() }"`) do set "REPO_ID=%%H"
if not defined REPO_ID (
    echo Failed to derive the repository's launch-output directory. 1>&2
    exit /b 1
)
if not defined LOCALAPPDATA (
    echo LOCALAPPDATA is required for isolated launch output. 1>&2
    exit /b 1
)
set "CARGO_TARGET_DIR=%LOCALAPPDATA%\%APP_NAME%\cargo-play\%REPO_ID%"
pushd "%APP_REPO_ROOT%" || exit /b 1
cargo build --release --locked -p prototype --manifest-path "%APP_REPO_ROOT%Cargo.toml"
set "RESULT=%ERRORLEVEL%"
if not "%RESULT%"=="0" goto finish
"%CARGO_TARGET_DIR%\release\prototype.exe" --assets "%APP_REPO_ROOT%assets" %*
set "RESULT=%ERRORLEVEL%"
:finish
popd
endlocal & exit /b %RESULT%
