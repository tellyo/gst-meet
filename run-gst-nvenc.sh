#!/usr/bin/env bash

gst-launch-1.0 -v \
  rtmpsrc location="rtmp://test-streamer-s3dev.aws-dev.intranet/stream_test/doom-eternal" ! \
  flvdemux name=d \
  d.video ! \
  queue ! \
  h264parse config-interval=-1 ! \
  nvh264dec ! \
  videorate ! \
  video/x-raw,framerate=30/1 ! \
  cudaupload ! \
  cudaconvert ! \
  'video/x-raw(memory:CUDAMemory),format=NV12' ! \
  nvcudah264enc \
    name=enkoder \
    tune=ultra-low-latency \
    bitrate=2500 \
    b-frames=0 \
    preset=p1 \
    rate-control=cbr \
    tune=3 \
    gop-size=30 \
    strict-gop=true \
    zero-reorder-delay=true \
    repeat-sequence-header=true \
    min-force-key-unit-interval=1000000000 \
    ! \
  h264parse config-interval=-1 ! \
  'video/x-h264,stream-format=byte-stream,profile=baseline' ! \
  queue ! \
  mpegtsmux \
    pat-interval=500 \
    pmt-interval=500 \
    pcr-interval=10 \
    name=mux \
  ! \
  queue ! \
  udpsink host=224.1.2.3 port=5321 sync=false