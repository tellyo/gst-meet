use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityLimitationDurations {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub none: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cpu: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bandwidth: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub other: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RtcOutboundRtpStreamStats {
  pub id: String,
  pub timestamp: f64,
  #[serde(rename = "type")]
  pub stats_type: String,
  pub ssrc: u32,
  pub kind: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub codec_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub transport_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub media_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bytes_sent: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub packets_sent: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub active: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub encoder_implementation: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub fir_count: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub frame_height: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub frame_width: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub frames_encoded: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub frames_per_second: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub frames_sent: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub header_bytes_sent: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub huge_frames_sent: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub key_frames_encoded: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub media_source_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub mid: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub nack_count: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub pli_count: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub power_efficient_encoder: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub qp_sum: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub quality_limitation_durations: Option<QualityLimitationDurations>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub quality_limitation_reason: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub quality_limitation_resolution_changes: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub remote_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub retransmitted_bytes_sent: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub retransmitted_packets_sent: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rtx_ssrc: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scalability_mode: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub target_bitrate: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub total_encode_time: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub total_encoded_bytes_target: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub total_packet_send_delay: Option<f64>,
}

impl RtcOutboundRtpStreamStats {
  pub(crate) fn new(ssrc: u32, kind: &'static str, timestamp: f64) -> Self {
    Self {
      id: format!("outbound-rtp-{kind}-{ssrc}"),
      timestamp,
      stats_type: "outbound-rtp".to_owned(),
      ssrc,
      kind: kind.to_owned(),
      codec_id: None,
      transport_id: Some("transport-0".to_owned()),
      media_type: Some(kind.to_owned()),
      bytes_sent: None,
      packets_sent: None,
      active: Some(true),
      encoder_implementation: None,
      fir_count: None,
      frame_height: None,
      frame_width: None,
      frames_encoded: None,
      frames_per_second: None,
      frames_sent: None,
      header_bytes_sent: None,
      huge_frames_sent: None,
      key_frames_encoded: None,
      media_source_id: None,
      mid: None,
      nack_count: None,
      pli_count: None,
      power_efficient_encoder: None,
      qp_sum: None,
      quality_limitation_durations: None,
      quality_limitation_reason: None,
      quality_limitation_resolution_changes: None,
      remote_id: None,
      retransmitted_bytes_sent: None,
      retransmitted_packets_sent: None,
      rtx_ssrc: None,
      scalability_mode: None,
      target_bitrate: None,
      total_encode_time: None,
      total_encoded_bytes_target: None,
      total_packet_send_delay: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn outbound_stats_use_webrtc_json_names() {
    let mut stats = RtcOutboundRtpStreamStats::new(1234, "video", 42.0);
    stats.bytes_sent = Some(10);
    stats.header_bytes_sent = Some(20);
    stats.rtx_ssrc = Some(5678);

    let json = serde_json::to_value(stats).unwrap();

    assert_eq!(json["type"], "outbound-rtp");
    assert_eq!(json["bytesSent"], 10);
    assert_eq!(json["headerBytesSent"], 20);
    assert_eq!(json["rtxSsrc"], 5678);
    assert!(json.get("codecId").is_none());
  }
}
