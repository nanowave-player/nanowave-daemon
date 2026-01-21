use std::io::Write;
use std::fs::OpenOptions;
use std::path::Path;

pub fn update_brightness(brightness_target_value: i32) {
    let path = Path::new("/sys/class/pwm/pwmchip8/pwm2/duty_cycle");
    if path.exists() {
        let mut file = OpenOptions::new()
            .write(true) // <--------- this
            .open(path)
            .ok()
            .unwrap();
        let _ = write!(file, "{}", brightness_target_value);
    }
}

pub fn brightness_percent_to_target_value(brightness_percent: f32) -> i32 {
    (brightness_percent * 2500f32).round() as i32
}