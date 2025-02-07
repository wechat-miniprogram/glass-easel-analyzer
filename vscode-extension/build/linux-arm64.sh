#!/bin/bash

cd $(dirname "$0")
cd ..

source build/common/package.sh linux-arm64 aarch64-unknown-linux-gnu --pre-release
