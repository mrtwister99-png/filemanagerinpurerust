#!/bin/bash
set -e
cd "$(dirname "$0")/../apps/filemanager"
cargo tauri dev
