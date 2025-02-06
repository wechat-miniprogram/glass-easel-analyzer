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

source build/common/package.sh darwin-arm64 aarch64-apple-darwin
