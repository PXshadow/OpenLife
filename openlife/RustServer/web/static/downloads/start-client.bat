@echo off
cd /d "%~dp0"
if exist "OneLifeData7\objects" set OHOL_CONTENT_DIR=%~dp0OneLifeData7
if exist "..\OneLifeData7\objects" set OHOL_CONTENT_DIR=%~dp0..\OneLifeData7
start "" "%~dp0ohol-client.exe"
