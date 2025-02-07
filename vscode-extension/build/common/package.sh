#!/bin/bash

TARGET_NAME=$1
TARGET_TRIPLE=$2
ARGS=$3

# build language server for different platforms
cd ..
echo "Building language server for ${TARGET_NAME}..."
if cargo build --target ${TARGET_TRIPLE} --release; then
  echo "Cargo build done."
else
  echo "Cargo build failed! Abort."
  exit -1
fi
cd vscode-extension

# copy resources
mkdir -p dist
cp ../target/${TARGET_TRIPLE}/release/glass-easel-analyzer dist/
cp ../backend-configuration/web/web.toml dist/

# packaging
mkdir -p packages
npx vsce package --target ${TARGET_NAME} -o packages ${ARGS}
