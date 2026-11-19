@echo off
cd /d "%~dp0..\apps\filemanager"
cargo tauri dev
pause
