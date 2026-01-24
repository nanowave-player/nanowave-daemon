use std::time::Duration;
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedReceiver;
use crate::api_command::{ApiCommand, ApiEvent};
use crate::media_source::file_media_source::FileMediaSource;
use crate::media_source::media_source::MediaSource;
use crate::player::player::Player;

pub struct Api {
    player: Player,
    media_source: FileMediaSource,
    cmd_rx: UnboundedReceiver<ApiCommand>,
    evt_tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
}

impl Api {
    pub fn new(
        player: Player,
        media_source: FileMediaSource,
        cmd_rx: UnboundedReceiver<ApiCommand>,
        evt_tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ) -> Self {
        Self {
            player,
            media_source,
            cmd_rx,
            evt_tx,
        }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ApiCommand::PlayerPlayMedia { id, reply } => {
                    //let _ = self.player.play_media(id).await;
                    let _ = reply.send(ApiEvent::Ok(json!(true)));
                }

                ApiCommand::PlayerPlay { reply } => {
                    // self.player.play();
                    let _ = reply.send(ApiEvent::Ok(json!(true)));
                }

                ApiCommand::PlayerPause { reply } => {
                    // self.player.pause();
                    let _ = reply.send(ApiEvent::Ok(json!(true)));
                }

                ApiCommand::PlayerSeek { pos, reply } => {
                    // self.player.try_seek(Duration::from_secs(pos));
                    let _ = reply.send(ApiEvent::Ok(json!(true)));
                }

                ApiCommand::MediaSourceFilter { query, reply } => {
                    // let res = self.media_source.filter(&query).await;
                    // let _ = reply.send(ApiEvent::Ok(json!(res)));
                }

                ApiCommand::MediaSourceFind { id, reply } => {
                    // let res = self.media_source.find(&id).await;
                    // let _ = reply.send(ApiEvent::Ok(json!(res)));
                }
            }
        }
    }

    /// Used by Player to emit async events
    pub fn emit_event(&self, method: &str, params: Value) {
        let _ = self.evt_tx.send(ApiEvent::PlaybackEvent {
            method: method.to_string(),
            params,
        });
    }
}
