use evdev::Device;

#[derive(Debug)]
pub enum GpioButtonEvent {
    ButtonPressed(u32),
    ButtonReleased(u32),
}

pub struct GpioButtonService {
    device: Device
}