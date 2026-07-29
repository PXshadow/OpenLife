@echo off
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d C:\OhOl\OpenLife\openlife\RustServer\crates\ol-sim
python _apply_move_nest_speed.py
if errorlevel 1 exit /b 1
cd /d C:\OhOl\OpenLife\openlife\RustServer
cargo test -p ol-sim --lib -- move_speed_held_nest -- --nocapture
exit /b %ERRORLEVEL%
