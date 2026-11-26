@echo off
cd /d "%~dp0"
where cargo >nul 2>&1 || (echo Install Rust from https://rustup.rs/ & pause & exit /b 1)
cargo build --release
if errorlevel 1 (pause & exit /b 1)
start "" "%~dp0target\release\hatori_ai.exe"
