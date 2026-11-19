@echo off
setlocal
set "HERE=%~dp0"
echo == File manager in RUST - build Android APK (bez Android Studia) ==

if "%ANDROID_HOME%"=="" if exist "%LOCALAPPDATA%\Android\Sdk" set "ANDROID_HOME=%LOCALAPPDATA%\Android\Sdk"
if "%ANDROID_HOME%"=="" (
  echo CHYBA: nemas nastavene ANDROID_HOME, viz README.md
  pause
  exit /b 1
)

if "%NDK_HOME%"=="" if exist "%ANDROID_HOME%\ndk" (
  for /f "delims=" %%d in ('dir /b /ad /o-n "%ANDROID_HOME%\ndk" 2^>nul') do (
    if not defined NDK_HOME set "NDK_HOME=%ANDROID_HOME%\ndk\%%d"
  )
)

echo ANDROID_HOME=%ANDROID_HOME%
echo NDK_HOME=%NDK_HOME%

cargo tauri --version >nul 2>&1
if errorlevel 1 (
  echo CHYBA: chybi tauri-cli. Nainstaluj ho jednou:
  echo   cargo install tauri-cli --version ^^2
  pause
  exit /b 1
)

cd /d "%HERE%..\apps\filemanager"
if not exist "src-tauri\gen\android" (
  echo Prvni init Android projektu...
  cargo tauri android init
  if errorlevel 1 (
    pause
    exit /b 1
  )
)

python "%HERE%android_manifest_perms.py"
if errorlevel 1 py -3 "%HERE%android_manifest_perms.py"

echo Buildim APK...
cargo tauri android build %*
if errorlevel 1 (
  pause
  exit /b 1
)

echo.
echo Hotovo. APK:
echo   src-tauri\gen\android\app\build\outputs\apk\universal\debug\app-universal-debug.apk
echo Instalace do pripojeneho telefonu:
echo   adb install -r src-tauri\gen\android\app\build\outputs\apk\universal\debug\app-universal-debug.apk
pause
