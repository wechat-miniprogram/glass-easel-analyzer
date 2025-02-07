@echo off

set PWD=%cd%
cd /D "%~dp0"
cd ..

if not exist "packages" mkdir "packages"
del packages\*.vsix
npm install

cd /D "%PWD%"
