// https://w3c.github.io/webrtc-stats/#dom-rtcstats
type RTCStats struct {
	ID        string  `json:"id,omitempty"`        // required
	Timestamp float64 `json:"timestamp,omitempty"` // required, unix epoch in milliseconds, https://www.w3.org/TR/hr-time-3/#dom-domhighrestimestamp
	Type      string  `json:"type,omitempty"`      // required
}

// https://w3c.github.io/webrtc-stats/#dom-rtcrtpstreamstats
type RTCRtpStreamStats struct {
	RTCStats
	Ssrc        uint32  `json:"ssrc"`
	Kind        string  `json:"kind"`
	CodecId     *string `json:"codecId,omitempty"`
	TransportId *string `json:"transportId"`
}

type RTCOutboundRtpStreamStatsVideo struct {
	RTCRtpStreamStats

	MediaType                          *string                     `json:"mediaType,omitempty"`
	BytesSent                          *uint64                     `json:"bytesSent"`
	PacketsSent                        *uint64                     `json:"packetsSent"`
	Active                             *bool                       `json:"active"`
	EncoderImplementation              *string                     `json:"encoderImplementation"`
	FirCount                           *int                        `json:"firCount"`
	FrameHeight                        *uint                       `json:"frameHeight"`
	FrameWidth                         *uint                       `json:"frameWidth"`
	FramesEncoded                      *uint                       `json:"framesEncoded"`
	FramesPerSecond                    *int                        `json:"framesPerSecond"`
	FramesSent                         *uint                       `json:"framesSent"`
	HeaderBytesSent                    *uint64                     `json:"headerBytesSent"`
	HugeFramesSent                     *uint                       `json:"hugeFramesSent"`
	KeyFramesEncoded                   *uint                       `json:"keyFramesEncoded"`
	MediaSourceId                      *string                     `json:"mediaSourceId"`
	Mid                                *string                     `json:"mid"`
	NackCount                          *int                        `json:"nackCount"`
	PliCount                           *int                        `json:"pliCount"`
	PowerEfficientEncoder              *bool                       `json:"powerEfficientEncoder"`
	QpSum                              *uint64                     `json:"qpSum"`
	QualityLimitationDurations         *QualityLimitationDurations `json:"qualityLimitationDurations"`
	QualityLimitationReason            *string                     `json:"qualityLimitationReason"`
	QualityLimitationResolutionChanges *int                        `json:"qualityLimitationResolutionChanges"`
	RemoteId                           *string                     `json:"remoteId"`
	RetransmittedBytesSent             *uint64                     `json:"retransmittedBytesSent"`
	RetransmittedPacketsSent           *uint64                     `json:"retransmittedPacketsSent"`
	RtxSsrc                            *uint32                     `json:"rtxSsrc"`
	ScalabilityMode                    *string                     `json:"scalabilityMode"`
	TargetBitrate                      *uint64                     `json:"targetBitrate"`
	TotalEncodeTime                    *float64                    `json:"totalEncodeTime"`
	TotalEncodedBytesTarget            *int                        `json:"totalEncodedBytesTarget"`
	TotalPacketSendDelay               *float64                    `json:"totalPacketSendDelay"`
}