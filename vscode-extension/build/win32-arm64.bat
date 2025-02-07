@echo off

set PWD=%cd%
cd /D "%~dp0"
cd ..

call build/common/package.bat win32-arm64 aarch64-pc-windows-msvc --pre-release

cd /D "%PWD%"
