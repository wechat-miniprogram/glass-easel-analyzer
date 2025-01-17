#!/bin/bash

# try to find a proper working directory
if [[ ${PWD} == */vscode-extension/build ]]; then
  cd ..
  echo "Switch to vscode extension directory: ${PWD}"
elif [[ ${PWD} == */vscode-extension ]]; then
  echo "In vscode extension directory: ${PWD}"
elif cd vscode-extension; then
  echo "Switch to vscode extension directory: ${PWD}"
else
  echo "Cannot find a proper vscode-extension directory."
  exit -1
fi

# build language server for different platforms
cd ..
echo "Building language server for darwin-arm64..."
cargo build --target aarch64-apple-darwin --release
cd vscode-extension

# copy resources
mkdir -p dist
cp ../target/aarch64-apple-darwin/release/glass-easel-analyzer dist/
cp ../backend-configuration/web/web.toml dist/

# package
vsce package --target darwin-arm64 -o /tmp
