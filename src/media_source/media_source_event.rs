use media_source::media_source_item::MediaSourceItem;

#[derive(Debug)]
pub enum MediaSourceEvent {
    FilterResults(Vec<MediaSourceItem>),
    FindResult(Option<MediaSourceItem>),
}
