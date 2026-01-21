use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    Press,
    Release,
}



const DEBOUNCE_DELAY: Duration = Duration::from_millis(400);

#[derive(Clone)]
pub struct ButtonHandler {
    state: Arc<Mutex<State>>,
}

#[derive(Debug)]
struct State {
    is_down: bool,
    clicks: u32,
}

impl ButtonHandler {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                is_down: false,
                clicks: 0,
            })),
        }
    }





    /// Single entry point
    pub fn handle_button_event(&self, action: ButtonAction) {
        let mut s = self.state.lock().unwrap();


        match action {
            ButtonAction::Press => {
                if !s.is_down {
                    s.is_down = true;
                    s.clicks += 1;
                }
            }
            ButtonAction::Release => s.is_down = false,
        }
    }
}