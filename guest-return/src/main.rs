// Copyright 2024 Amagi Poland

pub mod rtc;

use std::{collections::HashMap, time::Duration};
use anyhow::{bail, Context, Result, anyhow};
#[cfg(target_os = "macos")]
use cocoa::appkit::NSApplication;
use glib::object::ObjectExt;
use rtcp::payload_feedbacks::{full_intra_request::FullIntraRequest, picture_loss_indication::PictureLossIndication};
use structopt::StructOpt;
use tokio::{signal::ctrl_c, sync::mpsc, task, time::{sleep, timeout}};
use tracing::{error, info, trace, warn};
use gstreamer::{
  prelude::{ElementExt as _, ElementExtManual as _, GstBinExt as _, GstObjectExt, PadExt},
  GhostPad,
};
use http::Uri;
use http::uri;
use lib_gst_meet::{
    init_tracing, Authentication, Connection, JitsiConference, JitsiConferenceConfig, MediaType,
  };
use colibri::{ColibriMessage, Constraints, VideoType};
use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "macos"))]
#[tokio::main]
async fn main() -> Result<()> {
  main_inner().await
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(
  name = "guest-return",
  about = "Connect a GStreamer pipeline to a Jitsi Meet conference."
)]
struct Opt {
  #[structopt(short, long, parse(from_occurrences))]
  verbose: u8,

  #[structopt(
    long,
    env = "VIDEO_ADDRESS",
    help = "Multicast video input address in form ip:port",
  )]
  video_address: String,

  #[structopt(
    long,
    env = "VIDEO_PORT",
    help = "Multicast video input address in form ip:port",
  )]
  video_port: String,

  #[structopt(
    long,
    env = "AUDIO_CLIENT_NAME",
    help = "JACK audio client name",
  )]
  audio_client_name: String,

  #[structopt(
    long,
    env = "ROOM_NAME",
    help = "Name of the guest conference room. If not provided, and number of rooms for producer != 1, app will exit with error",
  )]
  room_name: String,

  #[structopt(
    long,
    default_value = "return.meet.jitsi",
  )]
  xmpp_domain: String,

  #[structopt(
    long,
    default_value = "muc.meet.jitsi",
    help = "If not specified, assumed to be conference.<xmpp-domain>"
  )]
  muc_domain: String,

  #[structopt(
    long,
    default_value = "focus.meet.jitsi",
    help = "If not specified, assumed to be focus@auth.<xmpp-domain>/focus"
  )]
  focus_jid: String,

  #[structopt(
    long,
    default_value = "h264",
    env = "VIDEO_CODEC",
    help = "The codec to transmit and receive video using. One of: av1, vp9, vp8, h264"
  )]
  video_codec: String,

  #[structopt(
    long,
    default_value = "StreamStudio",
    env = "NAME",
    help = "Name of the user we want to join as.",
  )]
  name: String,

  #[structopt(
    long,
    env = "SEND_PIPE",
    help = "gstreamer send pipeline"
  )]
  send_pipeline: Option<String>,

  #[structopt(
    long,
    default_value = "2000",
    env = "BUFFER_SIZE",
    help = "The size of the jitter buffers in milliseconds. Larger values are more resilient to packet loss and jitter, smaller values give lower latency."
  )]
  buffer_size: u32,

  #[structopt(
    long,
    env = "CONFERENCE_URL",
    help = "url to the conference, i.e. https://conference.tellyo.com"
  )]
  conference_url: String,

  #[structopt(long)]
  stereo: Option<bool>,
}

#[cfg(target_os = "macos")]
fn main() {
  // GStreamer requires an NSApp event loop in order for osxvideosink etc to work.
  let app = unsafe { cocoa::appkit::NSApp() };

  let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap();

  rt.spawn(async move {
    if let Err(e) = main_inner().await {
      error!("fatal: {:?}", e);
    }
    unsafe {
      cocoa::appkit::NSApp().stop_(cocoa::base::nil);
    }
    std::process::exit(0);
  });

  unsafe {
    app.run();
  }
}

#[derive(Debug)]
struct GuestReturn {
  conference_url: String,
  room_name: String,
  xmpp_domain: String,
  muc_domain: String,
  focus_jid: String,
  video_codec: String,
  video_address: String,
  video_port: String,
  audio_client_name: String,
  verbose: u8,
  name: String,
  send_pipeline: Option<String>,
  buffer_size: u32,
  websocket_uri: Uri,
  stereo: Option<bool>,
}

#[derive(Serialize, Deserialize,Debug,Clone)]
#[serde(rename_all = "camelCase")]
struct RoomDetails {
  #[serde(default)]
  conference_name: String,

  #[serde(default)]
  pretty_conference_name: String,

  #[serde(default)]
  shortened_ids: Option<Vec<String>>,

  #[serde(default)]
  hq_audio: Option<bool>,
}

//{"errors":[{"details":"room not found","status":404}]}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct JsonApiErrors {
  errors: Vec<JsonApiError>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct JsonApiError {
  details: String,
  status: u32,
}

fn init_gstreamer() -> Result<()> {
    trace!("starting gstreamer init");
    gstreamer::init()?;
    trace!("finished gstreamer init");
    Ok(())
  }

async fn main_inner() -> Result<()> {
    let opt = Opt::from_args();

    let mut config = GuestReturn{
      conference_url: opt.conference_url.clone(),
      room_name: opt.room_name,
      xmpp_domain: opt.xmpp_domain,
      muc_domain: opt.muc_domain,
      focus_jid: opt.focus_jid,
      video_codec: opt.video_codec,
      video_address: opt.video_address,
      video_port: opt.video_port,
      audio_client_name: opt.audio_client_name,
      verbose: opt.verbose,
      name: opt.name,
      send_pipeline: opt.send_pipeline,
      buffer_size: opt.buffer_size,
      websocket_uri: Uri::default(),
      stereo: opt.stereo,
    };
    
    let conference_domain = match opt.conference_url.clone().parse::<Uri>()?.into_parts().authority {
      Some(x) => x,
      None => return Err(anyhow!("Invalid conference url: {}: Should be in format https://conference.tellyo.com", opt.conference_url)),
    };

    config.websocket_uri = uri::Builder::new().scheme("wss").authority(conference_domain).path_and_query("/xmpp-websocket").build()?;

    println!("Starting guest with config {:#?}", config);

    start_guest(config).await
  }

async fn start_guest(config: GuestReturn) -> Result<()> {
    let send_video_height = 1080;
    let recv_video_scale_height = 720;
    let recv_video_scale_width = 1280;
    let video_type = "camera";
  
    init_tracing(match config.verbose {
      0 => tracing::Level::INFO,
      1 => tracing::Level::DEBUG,
      _ => tracing::Level::TRACE,
    });

    glib::log_set_default_handler(glib::rust_log_handler);

    let _ = init_gstreamer();

    /*
    let mut jitsi = Jitsi::new(config.tellyo_env, config.lsp_id);
    
    let (room_name, mut room) = match config.room_name {
      Some(x) => {
        let room =   match jitsi.get_room_details(x.as_str()).await { // @TODO
          Ok(r) => r,
          Err(e) => panic!("Error gathering data for room {x}: {e:?}"),
        };
        
        (x, room)
      },
      None => {
        let rooms = match jitsi.get_lsp_guest_details().await {
          Ok(r) => r,
        Err(e) => panic!("Error gathering data {e:?}"), // @TODO dont panic
        };

        if rooms.len() != 1 {
          panic!("Producer must contain one room, if room_name opt not provided. Producer has {} rooms.", rooms.len());
        }

        let (x, y) = rooms.iter().next().unwrap();

        (x.clone(), y.clone())
      }
    };


    if !room.is_return_enabled() {
      println!("Return is disabled for this room ({}). Waiting until it is enabled", room_name);

      while !room.is_return_enabled() {
        thread::sleep(time::Duration::from_secs(30));

        room =  match jitsi.get_room_details(room_name.as_str()).await { // @TODO
          Ok(r) => r,
          Err(e) => panic!("Error gathering data for room {room_name}: {e:?}"),
        };
      }

      println!("Return for room {room_name} was re-enabled on LSP. Starting...")
    }
    */
    let send_pipeline = match config.send_pipeline {
      Some(pipeline) => { Some(pipeline) },
      None => { Some(generate_gst_pipeline_string(config.video_codec.clone(), config.video_address, config.video_port, config.audio_client_name)) }
    } 
    .as_ref()
    .map(|pipeline| gstreamer::parse::bin_from_description(pipeline, false))
    .transpose()
    .context("failed to parse send pipeline")?;

    let mut web_socket_url_parts = config.websocket_uri.into_parts();
    web_socket_url_parts.path_and_query = web_socket_url_parts
        .path_and_query
        .map(|path_and_query| {
            let mut qs: HashMap<String, String> = path_and_query
            .query()
            .map(serde_urlencoded::from_str)
            .transpose()?
            .unwrap_or_default();
            if !qs.contains_key("room") {
                qs.insert("room".to_owned(), config.room_name.clone()); // @TODO
            }
            Ok::<_, anyhow::Error>(
            format!(
            "{}?{}",
            path_and_query.path(),
            serde_urlencoded::to_string(&qs)?,
            )
            .parse()?,
        )
        })
    .transpose()?;

    let web_socket_url = Uri::from_parts(web_socket_url_parts)?;

    let (connection, background) = Connection::new(
        &web_socket_url.to_string(),
        config.xmpp_domain.as_str(),
        Authentication::Anonymous,
    config.room_name.as_str(),
    #[cfg(feature = "tls-insecure")]
    opt.tls_insecure,
    #[cfg(not(feature = "tls-insecure"))]
    false,
    )
    .await
    .context("failed to build connection")?;

   tokio::spawn(background);

   connection.connect().await?;

   let room_jid = format!(
    "{}@{}",
    config.room_name,
    config.muc_domain
   );

   let stereo = match config.stereo {
    Some(x) => x,
    None => {
      info!("Stereo not provided, obtaining value from backend");
      let url = format!("{}/roomdetails/{}", config.conference_url, config.room_name);
      let response = reqwest::get(url).await?;
      let response_text = response.text().await?;
      
      // Try to parse as RoomDetails first
      if let Ok(room_details) = serde_json::from_str::<RoomDetails>(&response_text) {
        let stereo = room_details.hq_audio.unwrap_or(false);
        info!("Stereo value from backend: {}", stereo);
        stereo
      } else if let Ok(json_errors) = serde_json::from_str::<JsonApiErrors>(&response_text) {
        // Handle JsonApiErrors
        for error in json_errors.errors {
          error!("API Error: {} (status: {})", error.details, error.status);
        }

        // there's no room with this ID, sleep for 10 seconds and exit app
        sleep(Duration::from_secs(10)).await;
        return Err(anyhow::anyhow!("Failed to get room details from backend"));
      } else {
        return Err(anyhow::anyhow!("Failed to parse response from backend"));
      }
    },
   };

   let config = JitsiConferenceConfig{
    muc: room_jid.parse()?,
    focus: config.focus_jid.parse()?,
    nick: config.name,
    region: None,
    video_codec: config.video_codec,
    extra_muc_features: vec![],
    start_bitrate: 800,
    stereo: stereo,
    recv_video_scale_height: recv_video_scale_height,
    recv_video_scale_width: recv_video_scale_width,
    buffer_size: config.buffer_size,
   };

  let main_loop = glib::MainLoop::new(None, false);
  let conference = JitsiConference::join(connection, main_loop.context(), config)
    .await
    .context("failed to join conference")?;
  
  conference
    .set_send_resolution(send_video_height.into())
    .await;

  conference
    .send_colibri_message(ColibriMessage::ReceiverVideoConstraints {
      last_n: Some(0),
      selected_endpoints: None,
      on_stage_endpoints: None,
      default_constraints: Some(Constraints {
        max_height: Some(recv_video_scale_height.into()),
        ideal_height: None,
      }),
      constraints: None,
    })
    .await?;

    conference
      .send_colibri_message(ColibriMessage::VideoTypeMessage {
        video_type: match video_type {
          "camera" => VideoType::Camera,
          "desktop" => VideoType::Desktop,
          other => bail!(format!("invalid video type: {}", other)),
        },
    })
    .await?;

    if let Some(bin) = send_pipeline {
        conference.add_bin(&bin).await?;

        if let Some(audio) = bin.by_name("audio") {
            info!("Found audio element in pipeline, linking...");
            let audio_sink = conference.audio_sink_element().await?;
            audio.link(&audio_sink)?;
        }
        else {
            conference.set_muted(MediaType::Audio, true).await?;
        }

        if let Some(video) = bin.by_name("video") {
            info!("Found video element in pipeline, linking...");
            let video_sink = conference.video_sink_element().await?;
            video.link(&video_sink)?;
        }
        else {
            conference.set_muted(MediaType::Video, true).await?;
        }

        let pipeline = conference.pipeline().await?;
        let pli_injector = pipeline.by_name("pli-injector").unwrap();
        pipeline.by_name("return_rtpbin").map(|bin| {
          bin.connect_pad_added(move |bin, pad| {
            info!("Pad added: {:?}", pad);
            let pad_name = pad.name();
            if pad_name.starts_with("recv_rtp_src_0") {
              let identity_sink = pli_injector.static_pad("sink").unwrap();
              if identity_sink.is_linked() {
                println!("Unlinking existing link");
                if let Some(peer) = identity_sink.peer() {
                  println!("🔌 Unlinking previously linked pad: {}", peer.name());
      
                  peer.unlink(&identity_sink).unwrap_or_else(|err| {
                    eprintln!("Failed to unlink pad: {}", err);
                  });
                }
              }
      
              match pad.link(&identity_sink) {
                  Ok(_) => println!("Linked recv_rtp_src_0 to identity"),
                  Err(e) => eprintln!("Failed to link pad: {}", e),
              }
          }
            inspect_element_pads(&pli_injector);
            inspect_element_pads(&bin);
          })
        });

        let make_rtcp_logger = |direction: &'static str| {
          move |values: &[glib::Value]| -> Option<glib::Value> {
            let f = || {
              let buffer: gstreamer::Buffer = values[1].get()?;
              let mut buf = [0u8; 1500];
              buffer
                .copy_to_slice(0, &mut buf[..buffer.size()])
                    .map_err(|_| anyhow::anyhow!("invalid RTCP packet size"))?;
              let decoded = rtcp::packet::unmarshal(&mut &buf[..buffer.size()])?;
              for rtcp_packet in decoded {
                if let Some(e) = rtcp_packet.as_any().downcast_ref::<PictureLossIndication>() {
                  info!("RTCP {} size={}\n{:#?}", direction, buffer.size(), e);
                } else if let Some(e) = rtcp_packet.as_any().downcast_ref::<FullIntraRequest>() {
                  info!("RTCP {} size={}\n{:#?}", direction, buffer.size(), e);
                }
              }
              //println!("RTCP {} size={}\n{:#?}", direction, buffer.size(), decoded,);
    
              Ok::<_, anyhow::Error>(())
    
            };
            if let Err(e) = f() {
              println!("RTCP {}: {:?}", direction, e);
            }
            None
          }
        };
    
        pipeline
          .by_name("rtcp_logger_sender")
          .unwrap()
          .connect("handoff", false, make_rtcp_logger("SEND"));
    
        pipeline
          .by_name("rtcp_logger_receiver")
          .unwrap()
          .connect("handoff", false, make_rtcp_logger("RECV"));
    }
    else {
        conference.set_muted(MediaType::Audio, true).await?;
        conference.set_muted(MediaType::Video, true).await?;
    }
  
  conference
    .on_participant(move |conference, participant| {
        Box::pin(async move{
            info!("New participant: {:?}", participant);
            Ok(())
        })
    })
    .await;

    conference
        .on_participant_left(move |_conference, participant| {
            Box::pin(async move {
            info!("Participant left: {:?}", participant);
            Ok(())
        })
    })
    .await;

    conference
        .on_colibri_message(move |_conference, message| {
            Box::pin(async move {
            info!("Colibri message: {:?}", message);
            Ok(())
        })
    })
    .await;

    conference
        .set_pipeline_state(gstreamer::State::Playing)
        .await?;

    let conference_ = conference.clone();
    let main_loop_ = main_loop.clone();
    tokio::spawn(async move {
        ctrl_c().await.unwrap();

        info!("Exiting...");

        match timeout(Duration::from_secs(10), conference_.leave()).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => warn!("Error leaving conference: {:?}", e),
            Err(_) => warn!("Timed out leaving conference"),
        }

        main_loop_.quit();
    });

    let conference2 = conference.clone();
    let main_loop2 = main_loop.clone();
    tokio::spawn(async move {
      let (tx, mut rx) = mpsc::channel::<()>(1);

      match conference2.subscribe_to_colibri_recv_error(tx).await {
        Ok(()) => {},
        Err(e) => error!("{:?}", e)
      }

      rx.recv().await;

      info!("Exiting cause colibri error...");

      match timeout(Duration::from_secs(10), conference2.leave()).await {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => warn!("Error leaving conference: {:?}", e),
        Err(_) => warn!("Timed out leaving conference"),
      }

      main_loop2.quit();
    });

    task::spawn_blocking(move || main_loop.run()).await?;

    Ok(())
}

fn generate_gst_pipeline_string(codec: String, ip_video: String, port_video: String, audio_source: String) -> String {
  match codec.as_str() {
    "h264" => {
  format!("rtpbin name=return_rtpbin
    udpsrc reuse=true address={ip_video} port={port_video} caps=\"application/x-rtp, media=video, clock-rate=90000, encoding-name=H264, rtcp-fb-nack=1, rtcp-fb-nack-pli=(int)1, rtcp-fb-ccm-fir=1, payload=96\" ! return_rtpbin.recv_rtp_sink_0
    return_rtpbin. ! identity name=pli-injector ! rtph264depay ! h264parse config-interval=-1 ! queue name=video
    udpsrc reuse=true address={ip_video} port=5035 ! identity name=rtcp_logger_receiver ! return_rtpbin.recv_rtcp_sink_0
    return_rtpbin.send_rtcp_src_0 ! identity name=rtcp_logger_sender ! udpsink host=localhost port=5036 sync=false async=false
    jackaudiosrc connect=0 client-name={audio_source} !
    queue !
    rawaudioparse pcm-format=f32le sample-rate=48000 !
    audioconvert !
    queue !
    audioresample !
    audio/x-raw,channels=2,rate=48000 !
    queue !
    opusenc bitrate=100000 inband-fec=true name=audio")
    }
    "vp9" => {
  format!("rtpbin name=return_rtpbin
    udpsrc reuse=true address={ip_video} port={port_video} caps=\"application/x-rtp, media=video, clock-rate=90000, encoding-name=VP9, rtcp-fb-nack=1, rtcp-fb-nack-pli=(int)1, rtcp-fb-ccm-fir=1, payload=96\" ! return_rtpbin.recv_rtp_sink_0
    return_rtpbin. ! identity name=pli-injector ! rtpvp9depay ! vp9parse ! queue name=video
    udpsrc reuse=true address={ip_video} port=5035 ! identity name=rtcp_logger_receiver ! return_rtpbin.recv_rtcp_sink_0
    return_rtpbin.send_rtcp_src_0 ! identity name=rtcp_logger_sender ! udpsink host=localhost port=5036 sync=false async=false
    jackaudiosrc connect=0 client-name={audio_source} !
    queue !
    rawaudioparse pcm-format=f32le sample-rate=48000 !
    audioconvert !
    queue !
    audioresample !
    audio/x-raw,channels=2,rate=48000 !
    queue !
    opusenc bitrate=100000 inband-fec=true name=audio")

    }
    "av1" => {
  format!("rtpbin name=return_rtpbin
    udpsrc reuse=true address={ip_video} port={port_video} caps=\"application/x-rtp, media=video, clock-rate=90000, encoding-name=AV1, rtcp-fb-nack=1, rtcp-fb-nack-pli=(int)1, rtcp-fb-ccm-fir=1, payload=96\" ! return_rtpbin.recv_rtp_sink_0
    return_rtpbin. ! identity name=pli-injector ! rtpav1depay ! av1parse ! queue name=video
    udpsrc reuse=true address={ip_video} port=5035 ! identity name=rtcp_logger_receiver ! return_rtpbin.recv_rtcp_sink_0
    return_rtpbin.send_rtcp_src_0 ! identity name=rtcp_logger_sender ! udpsink host=localhost port=5036 sync=false async=false
    jackaudiosrc connect=0 client-name={audio_source} !
    queue !
    rawaudioparse pcm-format=f32le sample-rate=48000 !
    audioconvert !
    queue !
    audioresample !
    audio/x-raw,channels=2,rate=48000 !
    queue !
    opusenc bitrate=100000 inband-fec=true name=audio")
    }

    "vp8" => {
  format!("rtpbin name=return_rtpbin
    udpsrc reuse=true address={ip_video} port={port_video} caps=\"application/x-rtp, media=video, clock-rate=90000, encoding-name=VP8, rtcp-fb-nack=1, rtcp-fb-nack-pli=(int)1, rtcp-fb-ccm-fir=1, payload=96\" ! return_rtpbin.recv_rtp_sink_0
    return_rtpbin. ! identity name=pli-injector ! rtpvp8depay ! queue name=video
    udpsrc reuse=true address={ip_video} port=5035 ! identity name=rtcp_logger_receiver ! return_rtpbin.recv_rtcp_sink_0
    return_rtpbin.send_rtcp_src_0 ! identity name=rtcp_logger_sender ! udpsink host=localhost port=5036 sync=false async=false
    jackaudiosrc connect=0 client-name={audio_source} !
    queue !
    rawaudioparse pcm-format=f32le sample-rate=48000 !
    audioconvert !
    queue !
    audioresample !
    audio/x-raw,channels=2,rate=48000 !
    queue !
    opusenc bitrate=100000 inband-fec=true name=audio")
    }

    _ => panic!("Invalid codec: {}", codec),
  }
}

fn inspect_element_pads(element: &gstreamer::Element) {
  println!("🔍 Inspecting pads for element: {}", element.name());

  for pad in element.pads() {
      println!("▶ Pad: {}", pad.name());
      println!("  ↪ Direction: {:?}", pad.direction());

      if let Some(caps) = pad.current_caps() {
          println!("  ↪ Caps: {}", caps.to_string());
      } else {
          println!("  ↪ Caps: (none negotiated yet)");
      }

      if let Some(peer) = pad.peer() {
          println!("  ↪ Linked to: {} ({})", peer.name(), peer.parent_element().map(|p| p.name()).unwrap_or_else(|| "unknown".into()));
      } else {
          println!("  ↪ Linked to: (none)");
      }

      println!("  ↪ Active: {}", pad.is_active());
      println!();
  }
}