use serde_json::Value;
use tokio::sync::oneshot;

pub enum ApiCommand {
    PlayerPlayMedia {
        id: String,
        reply: oneshot::Sender<ApiEvent>,
    },
    PlayerPlay {
        reply: oneshot::Sender<ApiEvent>,
    },
    PlayerPause {
        reply: oneshot::Sender<ApiEvent>,
    },
    PlayerSeek {
        pos: u64,
        reply: oneshot::Sender<ApiEvent>,
    },

    MediaSourceFilter {
        query: String,
        reply: oneshot::Sender<ApiEvent>,
    },
    MediaSourceFind {
        id: String,
        reply: oneshot::Sender<ApiEvent>,
    },
}

pub enum ApiEvent {
    Ok(Value),
    Error(String),

    // async notifications (no reply expected)
    PlaybackEvent {
        method: String,
        params: Value,
    },
}
