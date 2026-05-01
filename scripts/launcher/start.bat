@echo off
setlocal enabledelayedexpansion
cd /d "%~dp0"
chcp 65001 >nul 2>&1
if not exist "assets\glb" mkdir "assets\glb"
if not exist "assets\stage" mkdir "assets\stage"
if not exist "assets\pmx" mkdir "assets\pmx"
if not exist "assets\vmd" mkdir "assets\vmd"
if not exist "assets\camera" mkdir "assets\camera"
if not exist "assets\music" mkdir "assets\music"
if not exist "assets\sync" mkdir "assets\sync"
if not exist "bin\terminal-miku3d.exe" (
  echo Error: Binary not found at bin\terminal-miku3d.exe
  pause
  exit /b 1
)
bin\terminal-miku3d.exe start %*
