#!/bin/bash

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "Error: This is not a Linux device."
    exit 1
fi

if [ ! -f deps.zip ]; then
    npm ci

    zip -rq deps.zip node_modules
fi

if [ -f node.zip ]; then
    exit 0
fi

rm -rf node-linux

mkdir node-linux
mkdir node-linux/include
mkdir node-linux/lib
mkdir node-linux/bin
mkdir node-linux/share

cp -r /usr/include/node node-linux/include/
cp -r /usr/local/lib/node_modules node-linux/lib/
cp -r /usr/local/bin/npm node-linux/bin/
cp -r /usr/local/bin/npx node-linux/bin/
cp -r /usr/bin/node node-linux/bin/
cp -r /usr/bin/corepack node-linux/bin/

zip -rq node.zip node-linux
