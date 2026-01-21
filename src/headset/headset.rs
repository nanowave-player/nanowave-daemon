pub struct Headset {

}

impl Headset {
    pub fn new() -> Headset {
        Self {
        }
    }


    pub async fn run(&mut self, device_path: String/*, evt_tx: mpsc::UnboundedSender<PlayerEvent>*/)/* -> JoinHandle<Result<String, String>> */{
        /*
        let btn_click_count = Arc::new(Mutex::new(0));
        let btn_click_count_clone = btn_click_count.clone();

        let btn_is_down = Arc::new(Mutex::new(false));
        let btn_is_down_clone = btn_is_down.clone();

        let debouncer = Debouncer::new(Duration::from_millis(500), DebounceMode::Trailing);
        let debouncer_clone = debouncer.clone();

        let event_device = device_path.clone();

        let handle = thread::spawn(move || {
            let mut device_option: Option<Device> = None;
            loop {
                if device_option.is_none() {
                    let device_path = Path::new(&event_device);
                    if !Path::exists(device_path) {
                        continue;
                    }

                    let device_result = Device::open(device_path);
                    if let Ok(d) = device_result {
                        device_option = Some(d);
                    }
                }

                if let Some(device) = &mut device_option {
                    for event in device.fetch_events().unwrap() {
                        // let _ = evt_tx.send(ev);

                        let mut trigger_action = false;
                        let mut event_str = "";
                        match event.destructure() {
                            EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 1) => {
                                // let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Press, ev.timestamp()));
                                // let _ = evt_tx.send(PlayerEvent::ExternalTrigger(TriggerAction::Toggle));
                                // println!("PLAYPAUSE PRESSED: {:?}", ev);
                                event_str = "PLAYPAUSE (PRESS)  ";
                            }
                            EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 0) => {
                                // let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Release, ev.timestamp()));
                                // let _ = evt_tx.send(PlayerEvent::ExternalTrigger(ButtonKey::PlayPause, ButtonAction::Release, ev.timestamp()));
                                // println!("PLAYPAUSE RELEASED: {:?}", ev);
                                event_str = "PLAYPAUSE (RELEASE)";
                            }
                            EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 1) => {
                                println!("VOLUME_UP PRESSED: {:?}", ev);
                                //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeUp, ButtonAction::Press, ev.timestamp()));
                            }
                            EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 0) => {
                                //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeUp, ButtonAction::Release, ev.timestamp()));
                                println!("VOLUME_UP RELEASED: {:?}", ev);
                            }
                            EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 1) => {
                                //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeDown, ButtonAction::Press, ev.timestamp()));
                                println!("VOLUME_DOWN PRESSED: {:?}", ev);
                            }
                            EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 0) => {
                                //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeDown, ButtonAction::Release, ev.timestamp()));
                                println!("VOLUME_DOWN RELEASED: {:?}", ev);
                            }
                            _ => {
                                // println!("got a different event: {:?}", event.destructure())
                            }
                        }
                        // let evt_tx_clone = evt_tx.clone();

                        // if trigger_action {}
                    }
                }
            }
        });

        loop {
            select! {
                    _ = debouncer.ready() => {
                        let mut clicks_guard = btn_click_count_clone.lock().unwrap();
                        let mut hold_guard = btn_is_down_clone.lock().unwrap();

                        // println!("debouncer ready | btn_repeat_count: {}, btn_state: {:?}", *clicks_guard, *hold_guard);


                        if *clicks_guard > 0 || *hold_guard {
                            println!("debouncer exec  | btn_repeat_count: {}, btn_state: {:?}", *clicks_guard, *hold_guard);
                        }

                        *clicks_guard = 0;
                        drop(clicks_guard);


                        // reset hold and clicks
                        *hold_guard = false;
                        drop(hold_guard);
                    }
            }


        }
*/

    }
}
