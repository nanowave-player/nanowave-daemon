use tokio::sync::oneshot;
use crate::media_source::media_source::MediaSourceEvent;

#[derive(Debug)]
pub enum MediaSourceCommand {
    Filter {
        query: String,
        reply: oneshot::Sender<MediaSourceEvent>,
    },
    Find {
        id: String,
        reply: oneshot::Sender<MediaSourceEvent>,
    },
}

