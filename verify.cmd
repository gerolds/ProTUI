@echo off
setlocal EnableExtensions DisableDelayedExpansion
pushd "%~dp0" || exit /b 1
set "CARGO_TARGET_DIR=%CD%\target"
cargo fmt --all --check
set "RESULT=%ERRORLEVEL%"
if not "%RESULT%"=="0" goto finish
cargo clippy --workspace --all-targets --locked -- -D warnings
set "RESULT=%ERRORLEVEL%"
if not "%RESULT%"=="0" goto finish
cargo test --workspace --locked
set "RESULT=%ERRORLEVEL%"
if not "%RESULT%"=="0" goto finish
cargo run --locked -p prototype -- --smoke
set "RESULT=%ERRORLEVEL%"
:finish
popd
endlocal & exit /b %RESULT%
