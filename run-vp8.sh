#!/usr/bin/env bash

VP8_COMMON_ARGS="\
    threads=8 \
    deadline=2 \
    cpu-used=8 \
    end-usage=cbr \
    keyframe-max-dist=30 \
    lag-in-frames=1 \
"

BUF1="\
    buffer-initial-size=500 \
    buffer-optimal-size=500 \
    buffer-size=500 \
"

BUF2="\
    buffer-initial-size=500 \
    buffer-optimal-size=500 \
    buffer-size=500 \
"

BUF3="\
    buffer-initial-size=500 \
    buffer-optimal-size=500 \
    buffer-size=500 \
"

TEMP_SCALABILITY_COMMON="\
    temporal-scalability-number-layers=3 \
    temporal-scalability-periodicity=4 \
    temporal-scalability-layer-sync-flags=<false,false,false,true> \
    temporal-scalability-rate-decimator={4,2,1} \
"

gst-launch-1.0 -v \
  rtpbin name=rtpbin latency=200 rtp-profile=avpf \
  \
  funnel name=rtp_funnell \
    ! application/x-rtp, media=video, encoding-name=VP8, payload=96, rtcp-fb-nack=1, rtcp-fb-nack-pli=1, rtcp-fb-ccm-fir=1 \
		! udpsink host=239.4.3.2 port=5322 \
  \
	funnel name=rtcp_funnell \
		! udpsink host=239.4.3.2 port=5323 sync=false async=false \
    \
  videotestsrc is-live=true pattern=smpte100 \
  ! video/x-raw,width=1280,height=720,framerate=30/1 \
  ! timecodestamper \
  ! clockoverlay \
  ! timeoverlay valignment=center halignment=center time-mode=time-code \
  ! tee name=video_tee \
  \
  video_tee. \
  ! queue name=qencoder0 \
  ! videoscale \
  ! "video/x-raw,format=I420,width=320,height=180" \
  ! vp8enc name=vp8enc_0 \
  $VP8_COMMON_ARGS \
    $BUF1 \
    $TEMP_SCALABILITY_COMMON \
    temporal-scalability-layer-id='{1,2,3}' \
    temporal-scalability-target-bitrate='{500,1000,2000}' \
  ! queue \
  ! rtpvp8pay pt=96 ssrc=123450 picture-id-mode=2 \
  ! rtprtxqueue max-size-time=1000 max-size-packets=0 \
  ! rtpbin.send_rtp_sink_0 \
  rtpbin.send_rtp_src_0 ! rtp_funnell.sink_0 \
  rtpbin.send_rtcp_src_0 ! rtcp_funnell.sink_0 \
  \
  video_tee. \
  ! queue name=qencoder1 \
  ! videoscale \
  ! "video/x-raw,format=I420,width=640,height=360" \
  ! vp8enc name=vp8enc_1 \
  $VP8_COMMON_ARGS \
    $BUF2 \
    $TEMP_SCALABILITY_COMMON \
    temporal-scalability-layer-id='{1,2,3}' \
    temporal-scalability-target-bitrate='{500,1000,2000}' \
  ! queue \
  ! rtpvp8pay pt=96 ssrc=123451 picture-id-mode=2 \
  ! rtprtxqueue max-size-time=1000 max-size-packets=0 \
  ! rtpbin.send_rtp_sink_1 \
  rtpbin.send_rtp_src_1 ! rtp_funnell.sink_1 \
  rtpbin.send_rtcp_src_1 ! rtcp_funnell.sink_1 \
  \
  video_tee. \
  ! queue name=qencoder2 \
  ! vp8enc name=vp8enc_2 \
  $VP8_COMMON_ARGS \
    $BUF3 \
    $TEMP_SCALABILITY_COMMON \
    temporal-scalability-layer-id='{1,2,3}' \
    temporal-scalability-target-bitrate='{750,1500,3000}' \
  ! queue \
  ! rtpvp8pay pt=96 ssrc=123452 picture-id-mode=2 \
  ! rtprtxqueue max-size-time=1000 max-size-packets=0 \
  ! rtpbin.send_rtp_sink_2 \
  rtpbin.send_rtp_src_2 ! rtp_funnell.sink_2 \
  rtpbin.send_rtcp_src_2 ! rtcp_funnell.sink_2 \
  udpsrc \
    reuse=true \
    name=udpsrc_rtcp \
    address=localhost \
    port=5324 \
  ! rtpbin.recv_rtcp_sink_0


exit $?

gst-launch-1.0 -v \
  videotestsrc is-live=true pattern=smpte100 ! \
  video/x-raw,width=1280,height=720,framerate=30/1 ! \
  timecodestamper ! \
  clockoverlay ! \
  timeoverlay valignment=center halignment=center time-mode=time-code ! \
  queue name=qencoder ! \
  vp8enc \
    threads=8 \
    deadline=2 \
    cpu-used=8 \
    end-usage=cbr \
    buffer-initial-size=500 \
    buffer-optimal-size=500 \
    buffer-size=500 \
    lag-in-frames=1 \
    temporal-scalability-number-layers=3 \
    temporal-scalability-periodicity=4 \
    temporal-scalability-layer-id='{1,2,3}' \
    temporal-scalability-layer-sync-flags='<false,false,false,true>' \
    temporal-scalability-rate-decimator='{4,2,1}' \
    temporal-scalability-target-bitrate='{750,1500,3000}' \
  ! \
  queue name=qpay ! \
  rtpvp8pay pt=96 picture-id-mode=2 ! \
  udpsink name=blabla host=224.1.2.3 port=5321 

exit $?

  queue name=qpay ! \
  rtpvp8pay pt=96 picture-id-mode=2 ! \
  udpsink name=blabla host=224.1.2.3 port=5321 
  exit $?

    temporal-scalability-number-layers=3 \
    temporal-scalability-periodicity=4 \
    temporal-scalability-layer-id='{1,2,3}' \
    temporal-scalability-layer-sync-flags='<false,false,false,true>' \
    temporal-scalability-rate-decimator='{4,2,1}' \
    temporal-scalability-target-bitrate='{750,1500,3000}' \



  vp8enc name=vp8enc_0 \
    threads=8 \
    deadline=2 \
    cpu-used=8 \
    end-usage=cbr \
    buffer-initial-size=500 \
    buffer-optimal-size=500 \
    buffer-size=500 \
    lag-in-frames=1 \
    target-bitrate=750 \
  ! \
  queue name=qpay ! \
  rtpvp8pay pt=96 ! \
  udpsink name=blabla host=224.1.2.3 port=5321 sync=false async=false

exit $?

    temporal-scalability-number-layers=3 \
    temporal-scalability-periodicity=4 \
    temporal-scalability-layer-id='{1,2,3}' \
    temporal-scalability-layer-sync-flags='<false,false,false,true>' \
    temporal-scalability-rate-decimator='{4,2,1}' \
    temporal-scalability-target-bitrate='{750,1500,3000}' \


  cudaupload ! \
  cudaconvert ! \
  video/x-raw(memory:CUDAMemory),format=NV12 ! \
  nvcudah264enc name=enkoder tune=ultra-low-latency bitrate=2000 b-frames=0 ! \
  video/x-h264,profile=baseline ! \
  h264parse ! \
  rtph264pay config-interval=1 pt=96 ! \
  application/x-rtp, media=video, encoding-name=H264, payload=96, rtcp-fb-nack=1, rtcp-fb-nack-pli=1, rtcp-fb-ccm-fir=1 ! \
  rtpbin.send_rtp_sink_0 \
  rtpbin.send_rtp_src_0 ! udpsink host=localhost port=31337 \
  rtpbin.send_rtcp_src_0 ! identity name=rtcp_logger_sender ! udpsink host=localhost port=31338 sync=false async=false \
  udpsrc address=localhost reuse=true port=31339 ! identity name=rtcp_logger_receiver ! rtpbin.recv_rtcp_sink_0



gst-launch-1.0 -v \
  rtmpsrc location="rtmp://test-streamer-s3dev.aws-dev.intranet/stream_test/singer" ! \
  flvdemux name=d \
  d.video ! \
  queue name=qdec ! \
  h264parse config-interval=-1 ! \
  openh264dec ! \
  videoconvert ! \
  queue name=qencoder ! \
  vp8enc name=vp8enc_0 \
    threads=8 \
    deadline=2 \
    cpu-used=8 \
    end-usage=cbr \
    keyframe-max-dist=30 \
    buffer-initial-size=500 \
    buffer-optimal-size=500 \
    buffer-size=500 \
    lag-in-frames=1 \
    target-bitrate=750 \
  ! \
  queue name=payudp ! \
  rtpvp8pay pt=96 ! \
  application/x-rtp, media=video, encoding-name=VP8, payload=96 ! \
  udpsink name=blabla host=224.1.2.3 port=5321 sync=false async=false

exit $?

  queue name=qscale ! \
  videoscale ! \
  "video/x-raw,width=320,height=180" ! \

exit $?

gst-launch-1.0 -v \
  rtmpsrc location="rtmp://test-streamer-s3dev.aws-dev.intranet/stream_test/singer" ! \
  flvdemux name=d \
  d.video ! \
  queue name=qdec ! \
  h264parse config-interval=-1 ! \
  openh264dec ! \
  videorate ! \
  video/x-raw,framerate=30/1 ! \
  queue name=qscale ! \
  videoscale ! \
  "video/x-raw,width=320,height=180" ! \
  queue name=qencoder ! \
  vp8enc name=vp8enc_0 \
    threads=8 \
    deadline=2 \
    cpu-used=8 \
    end-usage=cbr \
    keyframe-max-dist=30 \
    buffer-initial-size=500 \
    buffer-optimal-size=500 \
    buffer-size=500 \
    lag-in-frames=1 \
    target-bitrate=750 \
    temporal-scalability-number-layers=3 \
    temporal-scalability-periodicity=4 \
    temporal-scalability-layer-id='{1,2,3}' \
    temporal-scalability-layer-sync-flags='<false,false,false,true>' \
    temporal-scalability-rate-decimator='{4,2,1}' \
    temporal-scalability-target-bitrate='{750,1500,3000}' \
  ! \
  queue name=payudp ! \
  rtpvp8pay pt=100 ssrc=123450 picture-id-mode=2 ! \
  udpsink host=224.1.2.3 port=5321 sync=false async=false

exit $?

  tee name=video_tee \
  video_tee. ! \

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