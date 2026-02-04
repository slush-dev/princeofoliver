#!/usr/bin/env bash
set -euo pipefail
mkdir -p "/home/dev/projekty/princeofoliver/.xdg/config" \
  "/home/dev/projekty/princeofoliver/.xdg/cache" \
  "/home/dev/projekty/princeofoliver/.xdg/data"
XDG_CONFIG_HOME="/home/dev/projekty/princeofoliver/.xdg/config" \
XDG_CACHE_HOME="/home/dev/projekty/princeofoliver/.xdg/cache" \
XDG_DATA_HOME="/home/dev/projekty/princeofoliver/.xdg/data" \
"/home/dev/projekty/princeofoliver/tools/Godot_v4.6-stable_linux.x86_64" \
  --path "/home/dev/projekty/princeofoliver" \
  --rendering-driver opengl3 \
  --scene "res://scenes/Main.tscn" \
  -- \
  --labels
