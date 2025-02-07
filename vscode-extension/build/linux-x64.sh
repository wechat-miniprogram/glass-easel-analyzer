#!/bin/bash

cd $(dirname "$0")
cd ..

source build/common/package.sh linux-x64 x86_64-unknown-linux-gnu --pre-release
