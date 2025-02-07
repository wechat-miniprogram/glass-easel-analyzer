#!/bin/bash

cd $(dirname "$0")
cd ..

source build/common/package.sh darwin-x64 x86_64-apple-darwin --pre-release
