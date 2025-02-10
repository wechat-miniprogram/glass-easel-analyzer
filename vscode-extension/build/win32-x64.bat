@echo off

set PWD=%cd%
cd /D "%~dp0"
cd ..

call build/common/package.bat win32-x64 x86_64-pc-windows-msvc

cd /D "%PWD%"
