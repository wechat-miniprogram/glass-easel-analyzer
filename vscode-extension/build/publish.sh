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

VERSION=$1
if [[ ${VERSION} == "" ]]; then
  echo "VERSION is required: $0 [VERSION]"
  exit -1
fi

echo 'The following packages will be published:'
if ls packages/*-${VERSION}.vsix; then
  echo ''
else
  echo 'No proper packages found.'
  exit -1
fi

vsce publish --packagePath packages/*-${VERSION}.vsix
