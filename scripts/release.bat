@echo off
setlocal enabledelayedexpansion

:: --- デフォルト設定 ---
set BRANCH=main
set MESSAGE=chore: update version and push

:: --- 引数の解析 ---
:parse_args
if "%~1"=="" goto run_scripts
if /i "%~1"=="--branch" (
    set BRANCH=%~2
    shift
    shift
    goto parse_args
)
if /i "%~1"=="--message" (
    set MESSAGE=%~2
    shift
    shift
    goto parse_args
)
shift
goto parse_args

:run_scripts
echo [1/4] Running version update script...
python scripts/update_version.py
if %ERRORLEVEL% neq 0 (
    echo Error: update_version.py failed.
    exit /b %ERRORLEVEL%
)

echo [2/4] Git add...
git add .

echo [3/4] Git commit with message: "%MESSAGE%"...
git commit -m "%MESSAGE%"
if %ERRORLEVEL% neq 0 (
    echo No changes to commit or git error.
    goto push_stage
)

:push_stage
echo [4/4] Pushing to %BRANCH%...
git push origin %BRANCH%

echo.
echo Done! Factory is now working on GitHub Actions.
pause
