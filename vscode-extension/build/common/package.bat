@echo off

set TARGET_NAME=%1
set TARGET_TRIPLE=%2

:: build language server for different platforms
cd ..
echo Building language server for %TARGET_NAME%...
cargo build --target %TARGET_TRIPLE% --release
if %errorlevel%==0 (
    echo Cargo build done.
) else (
    echo Cargo build failed! Abort.
    exit /B -1
)
cd vscode-extension

:: copy resources
if not exist "dist" mkdir "dist"
copy "..\target\%TARGET_TRIPLE%\release\glass-easel-analyzer.exe" "dist\"
copy "..\backend-configuration\web\web.toml" "dist\"

:: packaging
npx vsce package --target %TARGET_NAME% -o packages
