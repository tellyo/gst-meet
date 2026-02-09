#!/usr/bin/env bash

gst-launch-1.0 -v \
  rtpbin name=rtpbin latency=200 rtp-profile=avpf \
	funnel name=rtp_funnell ! queue name=video \
	udpsrc \
      reuse=true \
      address=239.4.3.2 \
      port=5322 \
      caps="application/x-rtp, media=video, clock-rate=90000, encoding-name=VP8, rtcp-fb-nack=1, rtcp-fb-nack-pli=(int)1, rtcp-fb-ccm-fir=1, payload=96" \
    ! rtpbin.recv_rtp_sink_0 \
	rtpbin. \
    ! rtp_funnell.sink_0 \
	rtpbin. \
    ! rtp_funnell.sink_1 \
	rtpbin. \
    ! rtp_funnell.sink_2 \
    \
	udpsrc name=udpsrc_rtcp \
      reuse=true \
      address=239.4.3.2 \
      port=5323 \
    ! rtpbin.recv_rtcp_sink_0
    

    exit $?
    ! udpsink \
      name=udpsink_rtcp \
      host=localhost \
      port=5324 \
      sync=false \
      async=false

exit $?