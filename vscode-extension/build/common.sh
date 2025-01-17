#!/bin/bash

TARGET_NAME=$1
TARGET_TRIPLE=$2

# build language server for different platforms
cd ..
echo "Building language server for ${TARGET_NAME}..."
cargo build --target ${TARGET_TRIPLE} --release
cd vscode-extension

# copy resources
mkdir -p dist
cp ../target/${TARGET_TRIPLE}/release/glass-easel-analyzer dist/
cp ../backend-configuration/web/web.toml dist/

# package
vsce package --target ${TARGET_NAME} -o /tmp
