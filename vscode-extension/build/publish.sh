#!/bin/bash

cd $(dirname "$0")
cd ..

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

npx vsce publish --packagePath packages/*-${VERSION}.vsix --pre-release
