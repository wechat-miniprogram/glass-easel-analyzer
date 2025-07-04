#!/bin/bash

CWD=$(pwd)
cd $(dirname "$0")
cd ..

mkdir -p packages
rm packages/*.vsix

cd $CWD
