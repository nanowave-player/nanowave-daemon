use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time;

/* =======================
   External dependencies
   ======================= */

// Provided by your existing codebase
use crate::{Api, MediaSource};

/* =======================
   Player
   ======================= */

#[derive(Clone)]
pub struct Player {
    api: Arc<Api>,
    media_source: Arc<dyn MediaSource>,
    preferred_devices: Arc<Vec<String>>,
    state: Arc<Mutex<PlayerState>>,
}

struct PlayerState {
    // Must be kept alive for audio to play
    stream: Option<OutputStream>,
    sink: Option<Sink>,

    active_device_name: Option<String>,

    current_media_id: Option<String>,
    current_decoder: Option<Decoder<BufReader<File>>>,
}

impl Player {
    pub fn new(
        api: Arc<Api>,
        media_source: impl MediaSource + 'static,
        preferred_devices: Vec<&str>,
    ) -> Self {
        Self {
            api,
            media_source: Arc::new(media_source),
            preferred_devices: Arc::new(
                preferred_devices.into_iter().map(String::from).collect(),
            ),
            state: Arc::new(Mutex::new(PlayerState {
                stream: None,
                sink: None,
                active_device_name: None,
                current_media_id: None,
                current_decoder: None,
            })),
        }
    }

    /* =======================
       Background task
       ======================= */

    pub fn run_in_background(&self) {
        let player = self.clone();

        tokio::spawn(async move {
            loop {
                player.check_and_reconnect_device();
                time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    fn check_and_reconnect_device(&self) {
        let mut state = self.state.lock().unwrap();

        let device_ok = state
            .active_device_name
            .as_ref()
            .and_then(|name| find_device_by_name(name))
            .is_some();

        if device_ok {
            return;
        }

        if let Some((stream, sink, device_name)) =
            open_preferred_device(&self.preferred_devices)
        {
            state.stream = Some(stream);
            state.sink = Some(sink);
            state.active_device_name = Some(device_name.clone());

            self.api.emit_event(
                "player_device_changed",
                serde_json::json!({ "device": device_name }),
            );
        }
    }

    /* =======================
       Playback control
       ======================= */

    pub fn play_media(&self, id: String) -> anyhow::Result<()> {
        let path = self
            .media_source
            .open(&id)
            .ok_or_else(|| anyhow::anyhow!("Media not found"))?;

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let decoder = Decoder::new(reader)?;

        let mut state = self.state.lock().unwrap();
        let sink = state
            .sink
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No audio device"))?;

        sink.stop();
        sink.append(decoder.clone());
        sink.play();

        state.current_media_id = Some(id.clone());
        state.current_decoder = Some(decoder);

        self.api.emit_event(
            "player_started",
            serde_json::json!({ "id": id }),
        );

        Ok(())
    }

    pub fn play(&self) {
        if let Some(sink) = self.state.lock().unwrap().sink.as_ref() {
            sink.play();
            self.api.emit_event("player_play", serde_json::json!({}));
        }
    }

    pub fn pause(&self) {
        if let Some(sink) = self.state.lock().unwrap().sink.as_ref() {
            sink.pause();
            self.api.emit_event("player_pause", serde_json::json!({}));
        }
    }

    pub fn stop(&self) {
        if let Some(sink) = self.state.lock().unwrap().sink.as_ref() {
            sink.stop();
            self.api.emit_event("player_stop", serde_json::json!({}));
        }
    }

    pub fn seek(&self, pos: Duration) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();

        if let Some(decoder) = state.current_decoder.as_mut() {
            if decoder.try_seek(pos).is_ok() {
                self.api.emit_event(
                    "player_seek",
                    serde_json::json!({ "pos": pos.as_secs() }),
                );
                return Ok(());
            }
        }

        // Fallback: rebuild decoder
        let id = state
            .current_media_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No active media"))?;

        drop(state);
        self.play_media(id)?;
        Ok(())
    }
}

/* =======================
   Device helpers
   ======================= */

fn find_device_by_name(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.output_devices()
        .ok()?
        .find(|d| d.name().ok().as_deref() == Some(name))
}

fn open_preferred_device(
    preferred: &[String],
) -> Option<(rodio::OutputStream, rodio::Sink, String)> {
    let host = cpal::default_host();

    for name in preferred {
        let device = host
            .output_devices()
            .ok()?
            .find(|d| d.name().ok().as_deref() == Some(name))?;

        let config = device.default_output_config().ok()?;
        
        
        let (stream, handle) =
            rodio::OutputStream::try_from_device_config(&device, config).ok()?;

        let sink = rodio::Sink::new(&handle);

        return Some((stream, sink, name.clone()));
    }

    None
}

