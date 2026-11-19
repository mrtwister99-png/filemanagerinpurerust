#!/bin/bash
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"

echo "== File manager in RUST - build Android APK (bez Android Studia) =="

if [ -z "$ANDROID_HOME" ] && [ -d "$HOME/Android/Sdk" ]; then
  export ANDROID_HOME="$HOME/Android/Sdk"
fi
if [ -z "$ANDROID_HOME" ]; then
  echo "CHYBA: nemas nastavene ANDROID_HOME, viz README.md"
  exit 1
fi

if [ -z "$NDK_HOME" ] && [ -d "$ANDROID_HOME/ndk" ]; then
  latest="$(ls -1 "$ANDROID_HOME/ndk" | sort -V | tail -n 1)"
  if [ -n "$latest" ]; then
    export NDK_HOME="$ANDROID_HOME/ndk/$latest"
  fi
fi

echo "ANDROID_HOME=$ANDROID_HOME"
echo "NDK_HOME=$NDK_HOME"

if ! cargo tauri --version >/dev/null 2>&1; then
  echo "CHYBA: chybi tauri-cli. Nainstaluj ho jednou:"
  echo '  cargo install tauri-cli --version "^2"'
  exit 1
fi

cd "$HERE/../apps/filemanager"

if [ ! -d "src-tauri/gen/android" ]; then
  echo "Prvni init Android projektu..."
  cargo tauri android init
fi

if command -v python3 >/dev/null 2>&1; then
  python3 "$HERE/android_manifest_perms.py"
elif command -v python >/dev/null 2>&1; then
  python "$HERE/android_manifest_perms.py"
else
  echo "POZOR: python nenalezen, AndroidManifest.xml si oprávnění nedoplnil."
fi

echo "Buildim APK..."
cargo tauri android build "$@"

echo ""
echo "Hotovo. APK:"
echo "  src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
echo "Instalace do pripojeneho telefonu:"
echo "  adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
