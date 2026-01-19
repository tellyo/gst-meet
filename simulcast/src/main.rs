use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
#[cfg(target_os = "macos")]
use cocoa::appkit::NSApplication;
use colibri::{ColibriMessage, Constraints, VideoType};
use glib::object::ObjectExt as _;
use gstreamer::{
  GhostPad, prelude::{ElementExt as _, ElementExtManual, GstBinExt as _, GstBinExtManual, PadExt}
};
use http::Uri;
use lib_gst_meet::{
  init_tracing, Authentication, Connection, JitsiConference, JitsiConferenceConfig, MediaType,
};
use serde::Serialize;
use structopt::StructOpt;
use tokio::{signal::ctrl_c, task, time::{timeout, interval}, sync::mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, trace, warn};

#[derive(Debug, Serialize)]
struct InitMessage {
    #[serde(rename = "type")]
    message_type: String,
    data: InitData,
}

#[derive(Debug, Serialize)]
struct InitData {
    mic: bool,
    camera: bool,
    room: String,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "endpointId")]
    endpoint_id: String,
    token: String,
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(
  name = "gst-meet",
  about = "Connect a GStreamer pipeline to a Jitsi Meet conference."
)]
struct Opt {
  #[structopt(long)]
  web_socket_url: String,

  #[structopt(
    long,
    help = "If not specified, assumed to be the host part of <web-socket-url>"
  )]
  xmpp_domain: Option<String>,

  #[structopt(long)]
  room_name: String,

  #[structopt(
    long,
    help = "If not specified, assumed to be conference.<xmpp-domain>"
  )]
  muc_domain: Option<String>,

  #[structopt(
    long,
    help = "If not specified, assumed to be focus@auth.<xmpp-domain>/focus"
  )]
  focus_jid: Option<String>,

  #[structopt(long, help = "If not specified, anonymous auth is used.")]
  xmpp_username: Option<String>,

  #[structopt(long)]
  xmpp_password: Option<String>,

  #[structopt(long, help = "The JWT token for Jitsi JWT authentication")]
  xmpp_jwt: Option<String>,

  #[structopt(
    long,
    default_value = "vp8",
    help = "The codec to transmit and receive video using. One of: av1, vp9, vp8, h264"
  )]
  video_codec: String,

  #[structopt(long, default_value = "gst-meet")]
  nick: String,

  #[structopt(long)]
  token: String,

  #[structopt(long)]
  region: Option<String>,

  #[structopt(long)]
  send_pipeline: Option<String>,

  #[structopt(
    long,
    help = "A GStreamer pipeline which will be instantiated at startup. If an element named 'audio' is found, every remote participant's audio will be linked to it (and any 'audio' element in the recv-pipeline-participant-template will be ignored). If an element named 'video' is found, every remote participant's video will be linked to it (and any 'video' element in the recv-pipeline-participant-template will be ignored)."
  )]
  recv_pipeline: Option<String>,

  #[structopt(
    long,
    help = "A GStreamer pipeline which will be instantiated for each remote participant. If an element named 'audio' is found, the participant's audio will be linked to it. If an element named 'video' is found, the participant's video will be linked to it."
  )]
  recv_pipeline_participant_template: Option<String>,

  #[structopt(
    long,
    help = "Comma-separated endpoint IDs to select (prioritise receiving of)"
  )]
  select_endpoints: Option<String>,

  #[structopt(
    long,
    help = "The maximum number of video streams we would like to receive"
  )]
  last_n: Option<u16>,

  #[structopt(
    long,
    default_value = "720",
    help = "The maximum height we plan to send video at (used for stats only)."
  )]
  send_video_height: u16,

  #[structopt(
    long,
    help = "The video type to signal that we are sending. One of: camera, desktop"
  )]
  video_type: Option<String>,

  #[structopt(
    long,
    default_value = "1280",
    help = "The width to scale received video to before passing it to the recv-pipeline."
  )]
  recv_video_scale_width: u16,

  #[structopt(
    long,
    default_value = "720",
    help = "The height to scale received video to before passing it to the recv-pipeline. This will also be signalled as the maximum height that JVB should send video to us at."
  )]
  recv_video_scale_height: u16,

  #[structopt(
    long,
    default_value = "200",
    help = "The size of the jitter buffers in milliseconds. Larger values are more resilient to packet loss and jitter, smaller values give lower latency."
  )]
  buffer_size: u32,

  #[structopt(long)]
  start_bitrate: Option<u32>,

  #[structopt(long)]
  stereo: Option<bool>,

  #[structopt(short, long, parse(from_occurrences))]
  verbose: u8,

  #[cfg(feature = "tls-insecure")]
  #[structopt(
    long,
    help = "Disable TLS certificate verification (use with extreme caution)"
  )]
  tls_insecure: bool,

  #[cfg(feature = "log-rtp")]
  #[structopt(long, help = "Log all RTP packets at DEBUG level (extremely verbose)")]
  log_rtp: bool,

  #[cfg(feature = "log-rtp")]
  #[structopt(long, help = "Log all RTCP packets at DEBUG level")]
  log_rtcp: bool,
}

#[cfg(not(target_os = "macos"))]
#[tokio::main]
async fn main() -> Result<()> {
  main_inner().await
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

fn init_gstreamer() -> Result<()> {
  trace!("starting gstreamer init");
  gstreamer::init()?;
  trace!("finished gstreamer init");
  Ok(())
}

async fn main_inner() -> Result<()> {
  let opt = Opt::from_args();

  init_tracing(match opt.verbose {
    0 => tracing::Level::INFO,
    1 => tracing::Level::DEBUG,
    _ => tracing::Level::TRACE,
  });
  glib::log_set_default_handler(glib::rust_log_handler);

  init_gstreamer()?;

  // Parse pipelines early so that we don't bother connecting to the conference if it's invalid.

  // let send_pipeline = opt
  //   .send_pipeline
  //   .as_ref()
  //   .map(|pipeline| gstreamer::parse::bin_from_description(pipeline, false))
  //   .transpose()
  //   .context("failed to parse send pipeline")?;

  let recv_pipeline = opt
    .recv_pipeline
    .as_ref()
    .map(|pipeline| gstreamer::parse::bin_from_description(pipeline, false))
    .transpose()
    .context("failed to parse recv pipeline")?;

  let send_bin = Some(create_simulcast_bin()?);

  let mut web_socket_url: Uri = opt.web_socket_url.parse()?;
  let mut web_socket_url_parts = web_socket_url.into_parts();
  web_socket_url_parts.path_and_query = web_socket_url_parts
    .path_and_query
    .map(|path_and_query| {
      let mut qs: HashMap<String, String> = path_and_query
        .query()
        .map(serde_urlencoded::from_str)
        .transpose()?
        .unwrap_or_default();
      if !qs.contains_key("room") {
        qs.insert("room".to_owned(), opt.room_name.clone());
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

  web_socket_url = Uri::from_parts(web_socket_url_parts)?;

  let xmpp_domain = opt
    .xmpp_domain
    .as_deref()
    .or_else(|| web_socket_url.host())
    .context("invalid WebSocket URL")?;

  let (connection, background) = Connection::new(
    &web_socket_url.to_string(),
    xmpp_domain,
    match opt.xmpp_username {
      Some(username) => Authentication::Plain {
        username,
        password: opt
          .xmpp_password
          .context("if xmpp-username is provided, xmpp-password must also be provided")?,
      },
      None => match opt.xmpp_jwt {
        Some(token) => Authentication::Jwt { token },
        None => Authentication::Anonymous,
      },
    },
    &opt.room_name,
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
    opt.room_name,
    opt
      .muc_domain
      .clone()
      .unwrap_or_else(|| { format!("conference.{}", xmpp_domain) }),
  );

  let focus_jid = opt
    .focus_jid
    .clone()
    .unwrap_or_else(|| format!("focus@auth.{}/focus", xmpp_domain));

  let Opt {
    nick,
    region,
    video_codec,
    recv_pipeline_participant_template,
    send_video_height,
    recv_video_scale_height,
    recv_video_scale_width,
    buffer_size,
    start_bitrate,
    stereo,
    #[cfg(feature = "log-rtp")]
    log_rtp,
    #[cfg(feature = "log-rtp")]
    log_rtcp,
    ..
  } = opt;

  let config = JitsiConferenceConfig {
    muc: room_jid.parse()?,
    focus: focus_jid.parse()?,
    nick,
    region,
    video_codec,
    extra_muc_features: vec![],
    start_bitrate: start_bitrate.unwrap_or(800),
    stereo: stereo.unwrap_or_default(),
    recv_video_scale_height,
    recv_video_scale_width,
    buffer_size,
    #[cfg(feature = "log-rtp")]
    log_rtp,
    #[cfg(feature = "log-rtp")]
    log_rtcp,
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
      last_n: Some(opt.last_n.map(i32::from).unwrap_or(-1)),
      selected_endpoints: opt
        .select_endpoints
        .map(|endpoints| endpoints.split(',').map(ToOwned::to_owned).collect()),
      on_stage_endpoints: None,
      default_constraints: Some(Constraints {
        max_height: Some(opt.recv_video_scale_height.into()),
        ideal_height: None,
      }),
      constraints: None,
    })
    .await?;

  if let Some(video_type) = opt.video_type {
    conference
      .send_colibri_message(ColibriMessage::VideoTypeMessage {
        video_type: match video_type.as_str() {
          "camera" => VideoType::Camera,
          "desktop" => VideoType::Desktop,
          other => bail!(format!("invalid video type: {}", other)),
        },
      })
      .await?;
  }

  if let Some(bin) = send_bin {
    conference.add_bin(&bin).await?;

    if let Some(audio) = bin.by_name("audio") {
      info!("Found audio element in pipeline, linking...");
      let audio_sink = conference.audio_sink_element().await?;
      audio.link(&audio_sink)?;
    }
    else {
      conference.set_muted(MediaType::Audio, true).await?;
    }

    let video_sinks = conference.video_sink_elements().await?;
    // add all those three queues to the bin
    if let Some(queue) = bin.by_name("vp8enc_queue_1080p") {
      info!("Found video 1080p element in pipeline, linking...");
      queue.link(&video_sinks[0])?;
    }

    if let Some(queue) = bin.by_name("vp8enc_queue_720p") {
      info!("Found video 720p element in pipeline, linking...");
      queue.link(&video_sinks[1])?;
    }

    if let Some(queue) = bin.by_name("vp8enc_queue_360p") {
      info!("Found video 360p element in pipeline, linking...");
      queue.link(&video_sinks[2])?;
    }

    // if let Some(video) = bin.by_name("video") {
    //   info!("Found video element in pipeline, linking...");
    //   let video_sinks = conference.video_sink_elements().await?;
    //   video.link(&video_sinks[0])?;
    // }
    // else {
    //   conference.set_muted(MediaType::Video, true).await?;
    // }
  }
  else {
    conference.set_muted(MediaType::Audio, true).await?;
    conference.set_muted(MediaType::Video, true).await?;
  }

  if let Some(bin) = recv_pipeline {
    conference.add_bin(&bin).await?;

    if let Some(audio_element) = bin.by_name("audio") {
      info!(
        "recv pipeline has an audio element, a sink pad will be requested from it for each participant"
      );
      conference
        .set_remote_participant_audio_sink_element(Some(audio_element))
        .await;
    }

    if let Some(video_element) = bin.by_name("video") {
      info!(
        "recv pipeline has a video element, a sink pad will be requested from it for each participant"
      );
      conference
        .set_remote_participant_video_sink_element(Some(video_element))
        .await;
    }
  }

  conference
    .on_participant(move |conference, participant| {
      let recv_pipeline_participant_template = recv_pipeline_participant_template.clone();
      Box::pin(async move {
        info!("New participant: {:?}", participant);

        if let Some(template) = recv_pipeline_participant_template {
          let pipeline_description = template
            .replace(
              "{jid}",
              &participant
                .jid
                .as_ref()
                .map(|jid| jid.to_string())
                .unwrap_or_default(),
            )
            .replace(
              "{jid_user}",
              participant
                .jid
                .as_ref()
                .and_then(|jid| jid.node_str())
                .unwrap_or_default(),
            )
            .replace("{participant_id}", &participant.muc_jid.resource_str())
            .replace("{nick}", &participant.nick.unwrap_or_default());

          let bin = gstreamer::parse::bin_from_description(&pipeline_description, false)
            .context("failed to parse recv pipeline participant template")?;

          if let Some(audio_sink_element) = bin.by_name("audio") {
            let sink_pad = audio_sink_element.static_pad("sink").context(
              "audio sink element in recv pipeline participant template has no sink pad",
            )?;
            bin.add_pad(
              &GhostPad::builder_with_target(&sink_pad)?
                .name("audio")
                .build(),
            )?;
          }

          if let Some(video_sink_element) = bin.by_name("video") {
            let sink_pad = video_sink_element.static_pad("sink").context(
              "video sink element in recv pipeline participant template has no sink pad",
            )?;
            bin.add_pad(
              &GhostPad::builder_with_target(&sink_pad)?
                .name("video")
                .build(),
            )?;
          }

          bin.set_property(
            "name",
            format!("participant_{}", participant.muc_jid.resource()),
          );
          conference.add_bin(&bin).await?;
        }

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

  let conference3 = conference.clone();
  tokio::spawn(async move {
    // Wait until endpoint ID is available
    let endpoint_id = loop {
      match conference3.endpoint_id() {
        Ok(id) => {
          info!("Endpoint ID available: {}", id);
          break id.to_string();
        }
        Err(_) => {
          trace!("Waiting for endpoint ID to be available...");
          tokio::time::sleep(Duration::from_millis(100)).await;
        }
      }
    };
    
    // Connect to WebSocket
    let ws_url = "wss://conference-dev.tellyo.com/new-notify-ws";
    let (mut ws_sink, mut ws_stream) = match connect_async(ws_url).await {
      Ok((ws_stream, _)) => {
        info!("Connected to WebSocket at {}", ws_url);
        ws_stream.split()
      }
      Err(e) => {
        error!("Failed to connect to WebSocket at {}: {}", ws_url, e);
        return;
      }
    };
    
    // Create the JSON message
    let message = InitMessage {
      message_type: "init".to_string(),
      data: InitData {
        mic: false,
        camera: true,
        room: opt.room_name.clone(),
        display_name: "Alek1".to_string(),
        endpoint_id: endpoint_id.clone(),
        token: opt.token.clone(),
      },
    };
    
    // Serialize to JSON
    let json_message = match serde_json::to_string(&message) {
      Ok(json) => json,
      Err(e) => {
        error!("Failed to serialize init message: {}", e);
        return;
      }
    };
    
    // Set up interval for sending messages every second
    let mut interval = interval(Duration::from_secs(1));
    
    // Send initial message
    let message = Message::Text(json_message.clone());
    if let Err(e) = ws_sink.send(message).await {
      error!("Failed to send initial WebSocket message: {}", e);
      return;
    }
    info!("Sent initial init message with endpoint ID {} to WebSocket", endpoint_id);
    
    // Keep connection alive and send periodic messages
    loop {
      tokio::select! {
        // Send periodic messages
        _ = interval.tick() => {
          let message = Message::Text(json_message.clone());
          if let Err(e) = ws_sink.send(message).await {
            error!("Failed to send periodic WebSocket message: {}", e);
            break;
          }
          trace!("Sent periodic init message with endpoint ID {} to WebSocket", endpoint_id);
        }
        
        // Handle incoming messages (optional - just to keep connection alive)
        msg = ws_stream.next() => {
          match msg {
            Some(Ok(Message::Close(_))) => {
              info!("WebSocket connection closed by server");
              break;
            }
            Some(Ok(_)) => {
              // Ignore other messages for now
            }
            Some(Err(e)) => {
              error!("WebSocket error: {}", e);
              break;
            }
            None => {
              info!("WebSocket stream ended");
              break;
            }
          }
        }
      }
    }
    
    info!("WebSocket connection ended for endpoint ID: {}", endpoint_id);
  });
/*
  let conference2 = conference.clone();
  let conference3 = conference.clone();
  let main_loop__ = main_loop.clone();

  tokio::spawn(async move {
    let _ = conference2.pipeline_stopped().await;
    
    error!("Pipeline stopped, exiting...");

    match timeout(Duration::from_secs(10), conference3.leave()).await {
      Ok(Ok(_)) => {},
      Ok(Err(e)) => warn!("Error leaving conference: {:?}", e),
      Err(_) => warn!("Timed out leaving conference"),
    }

    main_loop__.quit();
  });
*/
/*
  let bus = conference.pipeline().await.unwrap().bus().context("failed to get pipeline bus")?;

  tokio::spawn(async move {
    let mut stream = bus.stream();

    while let Some(msg) = stream.next().await {
        match msg.view() {
          gstreamer::MessageView::Error(e) => {
            if let Some(d) = e.debug() {
              error!("{}", d);
            }
          },
          gstreamer::MessageView::Warning(e) => {
            if let Some(d) = e.debug() {
              warn!("{}", d);
            }
          },
          gstreamer::MessageView::StateChanged(state)
            if state.current() == gstreamer::State::Null =>
          {
            warn!("pipeline state is null. terminating...");
            //break;
          },
          _ => {},
        }
    }

    match timeout(Duration::from_secs(10), conference__.leave()).await {
      Ok(Ok(_)) => {},
      Ok(Err(e)) => warn!("Error leaving conference: {:?}", e),
      Err(_) => warn!("Timed out leaving conference"),
    }

    main_loop__.quit();
  });
*/
  task::spawn_blocking(move || main_loop.run()).await?;

  Ok(())
}


fn create_simulcast_bin() -> Result<gstreamer::Bin> {
  let bin = gstreamer::Bin::new();

  info!("Starting simulcast sender");

  // Video source: live SMPTE test pattern at 1080p30.
  let src = gstreamer::ElementFactory::make("videotestsrc")
      .name("src")
      .property("is-live", true)
      .property_from_str("pattern", "smpte")
      .build()
      .map_err(|err| anyhow!("failed to create videotestsrc: {err}"))?;

  let src_caps = gstreamer::Caps::builder("video/x-raw")
      .field("width", 1920i32)
      .field("height", 1080i32)
      .field("framerate", gstreamer::Fraction::new(30, 1))
      .build();

  let src_capsfilter = gstreamer::ElementFactory::make("capsfilter")
      .name("src_caps")
      .property("caps", &src_caps)
      .build()
      .map_err(|err| anyhow!("failed to create src capsfilter: {err}"))?;

  let tee = gstreamer::ElementFactory::make("tee")
      .name("tee")
      .build()
      .map_err(|err| anyhow!("failed to create tee: {err}"))?;

  bin
      .add_many([&src, &src_capsfilter, &tee])
      .map_err(|err| anyhow!("failed to add base elements: {err}"))?;
  
  gstreamer::Element::link_many([&src, &src_capsfilter, &tee])
      .map_err(|err| anyhow!("failed to link source chain: {err}"))?;

  // Build three simulcast branches (1080p, 720p, 360p).
  add_simulcast_branch(
      &bin, &tee, "1080p", 1920, 1080, 96, 0, false, 3000,
  )?;
  add_simulcast_branch(
      &bin, &tee, "720p", 1280, 720, 97, 1, true, 1500,
  )?;
  add_simulcast_branch(
      &bin, &tee, "360p", 640, 360, 98, 2, true, 750,
  )?;

  Ok(bin)
}

fn add_simulcast_branch(
  bin: &gstreamer::Bin,
  tee: &gstreamer::Element,
  label: &str,
  width: i32,
  height: i32,
  payload_type: i32,
  send_pad_index: u32,
  needs_scale: bool,
  bitrate: i32
) -> Result<()> {
  // Each branch produces one simulcast layer.
  let queue = gstreamer::ElementFactory::make("queue")
      .name(&format!("queue_{}", label))
      .build()
      .map_err(|err| anyhow!("failed to create queue for {label}: {err}"))?;

  let mut elements: Vec<gstreamer::Element> = vec![queue.clone()];

  info!("Creating simulcast branch for {label} with width {width}, height {height}, payload type {payload_type}, send pad index {send_pad_index}, needs scale {needs_scale}");

  if needs_scale {
      info!("Creating videoscale for {label}");
      let videoscale = gstreamer::ElementFactory::make("videoscale")
          .name(&format!("videoscale_{}", label))
          .build()
          .map_err(|err| anyhow!("failed to create videoscale for {label}: {err}"))?;
      let caps = gstreamer::Caps::builder("video/x-raw")
          .field("width", width)
          .field("height", height)
          .field("framerate", gstreamer::Fraction::new(30, 1))
          .build();
      let capsfilter = gstreamer::ElementFactory::make("capsfilter")
          .name(&format!("caps_{}", label))
          .property("caps", &caps)
          .build()
          .map_err(|err| anyhow!("failed to create branch capsfilter for {label}: {err}"))?;
      elements.push(videoscale);
      elements.push(capsfilter);
  }

  info!("Creating vp8enc for {label}");
  let vp8enc = gstreamer::ElementFactory::make("vp8enc")
      .name(&format!("vp8enc_{}", label))
      .property("threads", 8i32)
      .property("deadline", 2i64) // real-time
      .property("cpu-used", 8i32)
      .property("end-usage", GstVPXEncEndUsage::Cbr)
      .property("keyframe-max-dist", 30i32)
      .property("buffer-initial-size", 500i32)
      .property("buffer-optimal-size", 500i32)
      .property("buffer-size", 500i32)
      .property("lag-in-frames", 1i32)
      .property("target-bitrate", bitrate)
      .build()
      .map_err(|err| anyhow!("failed to create vp8enc for {label}: {err}"))?;
  
  let vp8enc_queue = gstreamer::ElementFactory::make("queue")
      .name(&format!("vp8enc_queue_{}", label))
      .build()
      .map_err(|err| anyhow!("failed to create queue for {label}: {err}"))?;

  elements.push(vp8enc);
  elements.push(vp8enc_queue);
  bin
      .add_many(elements.iter())
      .map_err(|err| anyhow!("failed to add branch elements for {label}: {err}"))?;

  gstreamer::Element::link_many(&elements)
      .map_err(|err| anyhow!("failed to link branch for {label}: {err}"))?;

  // Link tee -> branch queue.
  let tee_pad = tee
      .request_pad_simple("src_%u")
      .ok_or_else(|| anyhow!("failed to request tee pad for {label}"))?;
  let queue_sink = queue
      .static_pad("sink")
      .context("queue sink pad missing")?;
  tee_pad
      .link(&queue_sink)
      .map_err(|err| anyhow!("failed to link tee to queue for {label}: {err}"))?;

  info!(
      "Linked simulcast layer {} ({}x{}, PT {}, send_rtp_sink_{})",
      label, width, height, payload_type, send_pad_index
  );

  Ok(())
}