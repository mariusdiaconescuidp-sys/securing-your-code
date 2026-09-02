@echo off
set "RAD_DICTATE_PYTHON=%~dp0.runtime-venv\Scripts\python.exe"
set "RAD_DICTATE_SIDECAR=%~dp0sidecar\transcribe.py"
set "RAD_DICTATE_MODEL_PATH=%LOCALAPPDATA%\RadDictateWorkbench\models\large-v3"
set "PATH=C:\Program Files\nodejs;%PATH%"
npm run tauri dev
