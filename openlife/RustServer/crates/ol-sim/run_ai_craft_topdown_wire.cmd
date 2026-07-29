@echo off
cd /d "%~dp0"
python src\_apply_ai_craft_topdown.py
if errorlevel 1 python3 src\_apply_ai_craft_topdown.py
echo done
