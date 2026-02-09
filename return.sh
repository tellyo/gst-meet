#!/usr/bin/env bash
CC="gcc-10" cargo build -p guest-return && \
./target/debug/guest-return \
  --conference-url https://conference-dev.tellyo.com \
  --room-name roomaleek1-5qzgc476dm0x \
  --audio-client-name jack_capture-01 \
  --video-address 224.0.0.1 \
  --video-port 5034 \
  --video-codec vp8 \
  --name SimulStudio \
  --buffer-size 1000 \
  --number-of-layers 3 \
  --stereo false


