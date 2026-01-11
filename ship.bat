@echo off
wails build -nsis -platform windows
wails build -platform linux
python3 dist.py --zip --clean

