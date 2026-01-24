use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait};
use media_source::media_source_item::MediaSourceItem;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use rodio::source::SeekError;
use tokio::io;
use crate::Api;
use crate::media_source::media_source::MediaSource;

#[derive(Clone)]
pub struct Player {
    // api: Arc<Api>,
    media_source: Arc<dyn MediaSource>,
    preferred_devices: Arc<Vec<String>>,
    state: Arc<Mutex<PlayerState>>,
}

struct PlayerState {
    // Must be kept alive for audio to play
    stream: Option<OutputStream>,
    sink: Option<Arc<rodio::Sink>>,
    item: Option<MediaSourceItem>,
    active_device_name: Option<String>,
    current_decoder: Option<Decoder<BufReader<File>>>,
}

impl Player {
    pub fn new(
        // api: Arc<Api>,
        media_source: impl MediaSource + 'static,
        preferred_devices: Vec<&str>,
    ) -> Self {
        Self {
            // api,
            media_source: Arc::new(media_source),
            preferred_devices: Arc::new(
                preferred_devices.into_iter().map(String::from).collect(),
            ),
            state: Arc::new(Mutex::new(PlayerState {
                stream: None,
                sink: None,
                item: None,
                active_device_name: None,
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
                tokio::time::sleep(Duration::from_secs(30)).await;
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
            state.sink = Some(Arc::new(sink));
            state.active_device_name = Some(device_name.clone());

            /*
            self.api.emit_event(
                "player_device_changed",
                serde_json::json!({ "device": device_name }),
            );
            
             */
        }
    }

    fn sink_option(&self) -> Option<Arc<Sink>> {
        let mut state = self.state.lock().unwrap();
        state.item = None;
        state.sink.as_ref().map(Arc::clone)
    }

    pub async fn play_test(&self) {
        if let Some(sink) = self.sink_option() {
            sink.clear();

            let waves = [230.0, 270.0, 330.0, 270.0, 230.0];

            for w in waves {
                let source = rodio::source::SineWave::new(w).amplify(0.1);
                sink.append(source);
                sink.play();

                tokio::time::sleep(Duration::from_millis(200)).await;

                sink.stop();
                sink.clear();
            }
        }
    }



    pub async fn play_media(&mut self, id: String) -> io::Result<()> {
        let state = self.state.lock().unwrap();

        let self_item_option = state.item.as_ref();

        if let Some(item) = self_item_option
            && id == item.id
        {
            self.toggle();
            return Ok(());
        }


        let item_option = self.media_source.find(&id).await;
        if item_option.is_none() {
            return Ok(());
        }


        let item = item_option.unwrap();
        let path = Path::new(item.location.as_str());
        let file = File::open(path)?;

        if let Some(sink) = self.sink_option() {
            sink.clear();
            sink.append(rodio::Decoder::try_from(file).unwrap());
            sink.play();
        }

        Ok(())
    }

    pub fn play(&self) {
        if let Some(sink) = self.sink_option() {
            sink.play();
        }
    }


    pub fn pause(&self) {
        if let Some(sink) = self.sink_option() {
            sink.pause();
        }
    }

    pub fn toggle(&self) {
        if let Some(sink) = self.sink_option() {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }

    pub fn try_seek(&self, position: Duration) -> Result<(), SeekError> {
        if let Some(sink) = self.sink_option() {
            return sink.try_seek(position);
        }
        Ok(())
    }


}



fn find_device_by_name(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.output_devices()
        .ok()?
        .find(|d| d.name().ok().as_deref() == Some(name))
}


// fixed chatgpt variant
fn open_preferred_device(
    preferred: &[String],
) -> Option<(rodio::OutputStream, rodio::Sink, String)> {
    let host = cpal::default_host();
    for name in preferred {
        let device = host
            .output_devices()
            .ok()?
            .find(|d| {
                d.name()
                    .ok()
                    .map(|device_name| device_name == *name)
                    .unwrap_or(false)
            })?;

        // let config = device.default_output_config().ok()?;

        let builder_result = OutputStreamBuilder::from_device(device);
        if let Ok(builder) = builder_result {
            let stream = builder.open_stream_or_fallback().unwrap();
            let sink = Sink::connect_new(stream.mixer());
            return Some((stream, sink, name.clone()));
        }
    }
    None
}


/*
pub struct Player {
    media_source: Arc<dyn MediaSource>,
    preferred_device_name: String,
    fallback_device_name: String,
    stream: Option<OutputStream>, // when removed, the samples do not play
    sink: Option<Sink>,
    item: Option<MediaSourceItem>,
}



impl Player {
    // sink:Option<Sink>, stream: Option<OutputStream>
    pub fn new(
        media_source: Arc<dyn MediaSource>,
        preferred_device_name: String,
        fallback_device_name: String,
    ) -> Player {
        Self {
            media_source,
            preferred_device_name,
            fallback_device_name,
            stream: None,
            sink: None,
            item: None,
        }
    }
}

 */
