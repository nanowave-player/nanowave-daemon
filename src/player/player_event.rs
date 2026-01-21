use std::time::Duration;
use crate::player::trigger_action::TriggerAction;

#[derive(Debug)]
pub enum PlayerEvent {
    Status(String, String),
    Position(String, Duration),
    Stopped,
    ExternalTrigger(TriggerAction)
}