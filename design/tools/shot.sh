#!/bin/sh
# Renders one board page to a PNG for visual checks: design/tools/shot.sh <file.html> <out.png> [width] [height]
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 --virtual-time-budget=4000 --window-size="${3:-1540},${4:-1100}" --screenshot="$2" "file://$1" >/dev/null 2>&1
