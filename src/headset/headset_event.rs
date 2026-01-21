use crate::headset::headset_button::HeadsetButton;

pub enum HeadsetEvent {
    Press(HeadsetButton),
    Release(HeadsetButton),
}