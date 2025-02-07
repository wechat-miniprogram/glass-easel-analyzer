#!/bin/bash

cd $(dirname "$0")
cd ..

source build/common/package.sh linux-armhf armv7-unknown-linux-gnueabihf --pre-release
