use std::time::Duration;

#[derive(Debug)]
pub enum PlayerCommand {
    Update(String),
    PlayTest(),
    PlayMedia(String),
    Pause(),
    Stop(),
    Play(),
    Next(),
    Previous(),
    SeekRelative(i64),
    SeekTo(Duration),
}