// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs
// load multiple sources with rodio: https://stackoverflow.com/questions/75505017/how-can-i-make-rust-with-the-rodio-crate-load-multiple-sources-in-a-vec-so-i

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Device;
use rodio::source::SeekError;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use std::cmp::max;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use media_source::media_source_chapter::MediaSourceChapter;
use media_source::media_source_item::MediaSourceItem;
use mpsc::UnboundedReceiver;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use crate::media_source::media_source::MediaSource;
pub(crate) use crate::player::player_command::PlayerCommand;
pub(crate) use crate::player::player_event::PlayerEvent;

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

    pub fn connect_sink(&mut self) {
        let builder_option = Self::create_device_output_builder(
            self.preferred_device_name.clone(),
            self.fallback_device_name.clone(),
        );
        if let Some(builder) = builder_option {
            let stream = builder.open_stream_or_fallback().unwrap();
            self.sink = Some(Sink::connect_new(stream.mixer()));
            self.stream = Some(stream);
        }
    }

    fn previous_delay(&self) -> Duration {
        // if you are within this time of a track, it does not skip to 0 but to the previous track
        Duration::from_secs(3)
    }

    //                     let match_string = "USB-C to 3.5mm Headphone Jack A";
    //                     let match_string2 = "pipewire";
    fn create_device_output_builder(
        preferred_name: String,
        fallback_name: String,
    ) -> Option<OutputStreamBuilder> {
        let host = cpal::default_host();
        let devices = host.output_devices().unwrap();

        let device: Option<Device> = {
            let mut preferred_dev: Option<cpal::Device> = None;
            let mut fallback_dev: Option<cpal::Device> = None;
            let mut first_dev: Option<cpal::Device> = None;
            for d in devices {
                // println!("====={}", d.name().unwrap().to_string());
                if d.name().unwrap() == preferred_name {
                    preferred_dev = Some(d);
                    break;
                } else if d.name().unwrap() == fallback_name {
                    fallback_dev = Some(d);
                } else if first_dev.is_none() {
                    first_dev = Some(d)
                }
            }

            if preferred_dev.is_some() {
                preferred_dev
            } else if fallback_dev.is_some() {
                fallback_dev
            } else {
                first_dev
            }
        };

        let builder: Option<OutputStreamBuilder> = if device.is_some() {
            let selected_device = device.unwrap();
            let builder_result = OutputStreamBuilder::from_device(selected_device);
            Some(builder_result.unwrap())
        } else {
            None
        };

        builder
    }

    async fn play_test(&mut self) {
        if let Some(sink) = &self.sink {
            self.item = None;
            sink.clear();
            let waves = vec![230f32, 270f32, 330f32, 270f32, 230f32];
            for w in waves {
                let source = rodio::source::SineWave::new(w).amplify(0.1);
                sink.append(source);
                sink.play();
                sleep(Duration::from_millis(200)).await;
                sink.stop();
                sink.clear();
            }
        }
    }
    pub async fn play_media(&mut self, id: String) -> io::Result<()> {
        let self_item = self.item.clone();

        if let Some(i) = self_item
            && id == i.id
        {
            self.toggle();
            return Ok(());
        }

        self.item = self.media_source.find(&id).await;
        if self.item.is_none() {
            return Ok(());
        }
        let self_item = self.item.clone();
        let item = self_item.unwrap();
        let path = Path::new(item.location.as_str());
        let file = File::open(path)?;

        if let Some(sink) = &self.sink {
            sink.clear();
            sink.append(rodio::Decoder::try_from(file).unwrap());
            sink.play();
        }
        Ok(())
    }

    fn toggle(&self) {
        if let Some(sink) = &self.sink {
            if sink.is_paused() {
                sink.play()
            } else {
                sink.pause()
            }
        }
    }

    fn play(&self) {
        if let Some(sink) = &self.sink {
            sink.play();
        }
    }

    fn pause(&self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    fn try_seek(&self, position: Duration) -> Result<(), SeekError> {
        if self.sink.is_none() {
            return Ok(());
        }
        let sink = self.sink.as_ref().unwrap();
        sink.try_seek(position)
    }

    fn chapters(&self) -> Vec<MediaSourceChapter> {
        let self_item = self.item.clone();
        if self_item.is_none() {
            return vec![];
        }
        let current_item = self_item.unwrap();
        current_item.metadata.chapters
    }

    fn next_chapter(&self) -> Option<MediaSourceChapter> {
        if let Some(sink) = &self.sink {
            let current_pos = sink.get_pos();
            let chapters = self.chapters();
            for chapter in chapters {
                if chapter.start > current_pos {
                    return Some(chapter);
                }
            }
        }
        None
    }

    fn current_chapter(&self) -> Option<MediaSourceChapter> {
        if let Some(sink) = &self.sink {
            let current_pos = sink.get_pos();
            let chapters = self.chapters();
            if chapters.is_empty() {
                return None;
            }
            for chapter in chapters {
                if chapter.start <= current_pos && chapter.end() >= current_pos {
                    return Some(chapter);
                }
            }
        }
        None
    }

    fn previous_chapter(&self) -> Option<MediaSourceChapter> {
        if let Some(sink) = &self.sink {
            let current_pos = sink.get_pos();
            let chapters = self.chapters();
            if chapters.is_empty() {
                return None;
            }
            let mut last_chapter: Option<MediaSourceChapter> = None;
            for chapter in chapters {
                if chapter.start <= current_pos && chapter.end() >= current_pos {
                    break;
                }
                last_chapter = Some(chapter);
            }
            return last_chapter;
        }
        None
    }

    // todo:
    // next, previous, set_volume, set_speed

    pub async fn run(
        &mut self,
        mut cmd_rx: UnboundedReceiver<PlayerCommand>,
        evt_tx: UnboundedSender<PlayerEvent>,
    ) {
        let mut last_sink_update_attempt = SystemTime::now();
        loop {
            // polling in case the audio hardware has not been successfully initialized yet

            let now = SystemTime::now();

            if self.sink.is_none() && last_sink_update_attempt + Duration::from_millis(2000) < now {
                self.connect_sink();
                last_sink_update_attempt = now;
            }

            if let Some(sink) = &self.sink {
                tokio::select! {

                    // this part makes the UI crash
                    /*
                    Some(btn_cmd) = button_cmd_rx.recv() => {
                        match btn_cmd {
                            PlayerCommand::HandleButton(key,action,timestamp) => {
                                println!("===== handle button =====");
                            }
                            _ => {}
                        }
                    }

                     */

                    Some(cmd) = cmd_rx.recv() => {
                        println!("============== cmd received ==============");
                        match cmd {
                            PlayerCommand::Update(s) => {
                                self.play_media(s.clone()).await;
                                // format!("Playing {}", x)
                                // todo: implement player.is_playing / player.status

                                self.update_playing_status(&evt_tx).await;
                                /*
                                if self.sink.is_paused() {
                                    let _ = evt_tx.send(PlayerEvent::Status("paused".to_string()));
                                } else {
                                    let _ = evt_tx.send(PlayerEvent::Status("playing".to_string()));
                                }

                                 */
                            }
                            PlayerCommand::PlayTest() => {
                                self.play_test().await;
                            }
                            PlayerCommand::PlayMedia(s) => {
                                self.play_media(s).await;
                                self.update_playing_status(&evt_tx).await;
                            }
                            PlayerCommand::Play() => {
                                self.play();
                                self.update_playing_status(&evt_tx).await;
                            }
                            PlayerCommand::Pause() => {
                                self.pause();
                                self.update_playing_status(&evt_tx).await;
                            }
                            PlayerCommand::Stop() => {
                                let _ = evt_tx.send(PlayerEvent::Stopped);
                                break;
                            },
                            PlayerCommand::Next() => {
                                let next_chapter = self.next_chapter();
                                if next_chapter.is_some() {
                                    let new_pos = next_chapter.unwrap().start;
                                    self.try_seek(new_pos).unwrap();
                                    self.update_position(&evt_tx, new_pos).await;
                                } else {
                                    sink.skip_one()
                                }
                            }
                            PlayerCommand::Previous() => {
                                let current_pos = sink.get_pos();
                                if current_pos <= self.previous_delay() {
                                    // todo: skip to previous playlist item
                                    // return
                                }

                                if let Some(current_chapter) = self.current_chapter()
                                    && current_pos - current_chapter.start > self.previous_delay() {
                                    self.try_seek(current_chapter.start).unwrap();
                                    self.update_position(&evt_tx, current_chapter.start).await;

                                } else if let Some(previous_chapter) = self.previous_chapter() {
                                    self.try_seek(previous_chapter.start).unwrap();
                                    self.update_position(&evt_tx, previous_chapter.start).await;

                                } else {
                                    let zero = Duration::from_secs(0);
                                    self.try_seek(zero).unwrap();
                                    self.update_position(&evt_tx, zero).await;
                                }
                            }
                            PlayerCommand::SeekRelative(millis) => {
                                let new_pos = max(sink.get_pos().as_millis() as i64 + millis, 0) as u64;
                                self.try_seek(Duration::from_millis(new_pos));
                            }
                            PlayerCommand::SeekTo(_) => {},
                            _ => {}
                        }
                    }

                    _ = tokio::time::sleep(Duration::from_millis(500)) => {
                        self.update_position(&evt_tx, sink.get_pos()).await;
                    }
                }



            }
        }
    }

    /*
    pub async fn run_buttons(
        &mut self,
        mut cmd_rx: UnboundedReceiver<PlayerCommand>,
    ) {
        loop {
            if let Some(sink) = &self.sink {
                tokio::select! {
                    Some(cmd) = cmd_rx.recv() => {
                        println!("============== run_buttons cmd received ==============");
                        match cmd {
                            PlayerCommand::HandleButton(ButtonKey, ButtonAction, SystemTime) => {
                                if ButtonAction == ButtonAction::Release {
                                    self.toggle()
                                }
                            },
                            _ => {}
                        }
                    }

                }
            }
        }
    }
    */
    async fn update_position(&self, evt_tx: &mpsc::UnboundedSender<PlayerEvent>, pos: Duration) {
        if let Some(item) = self.item.clone() {
            let _ = evt_tx.send(PlayerEvent::Position(item.id.to_string(), pos));
        }
    }

    async fn update_playing_status(&self, evt_tx: &UnboundedSender<PlayerEvent>) {
        if let Some(sink) = &self.sink {
            let self_item_opt = self.item.clone();
            if self_item_opt.is_none() {
                return;
            }
            let self_item_opt = self.item.clone();
            if self_item_opt.is_none() {
                return;
            }
            let self_item = self_item_opt.unwrap();
            if sink.is_paused() {
                let _ = evt_tx.send(PlayerEvent::Status(
                    self_item.id.to_string(),
                    "paused".to_string(),
                ));
            } else {
                let _ = evt_tx.send(PlayerEvent::Status(
                    self_item.id.to_string(),
                    "playing".to_string(),
                ));
            }
        }
    }
}
