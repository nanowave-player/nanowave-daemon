use media_source::media_source_item::MediaSourceItem;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
pub(crate) use crate::media_source::media_source_command::MediaSourceCommand;
pub(crate) use crate::media_source::media_source_event::MediaSourceEvent;

#[async_trait::async_trait]
pub trait MediaSource: Send + Sync {
    fn id(&self) -> String;
    async fn filter(&self, query: &str) -> Vec<MediaSourceItem>;
    async fn find(&self, id: &str) -> Option<MediaSourceItem>;


    /// Async run loop - consumes self
    async fn run(
        self,
        cmd_rx: UnboundedReceiver<MediaSourceCommand>
    );
}
