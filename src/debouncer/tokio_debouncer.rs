use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio::time::Instant;


#[cfg(feature = "parking_lot")]
pub use parking_lot::{Mutex, MutexGuard};
#[cfg(not(feature = "parking_lot"))]
pub use std::sync::MutexGuard;
use std::time::Duration;


#[cfg(not(feature = "parking_lot"))]
pub trait MutexExt<T> {
    /// Lock the mutex, panicking if poisoned.
    fn risky_lock(&self) -> MutexGuard<T>;
}
#[cfg(not(feature = "parking_lot"))]
impl<T> MutexExt<T> for Mutex<T> {
    fn risky_lock(&self) -> MutexGuard<T> {
        self.lock().expect("Mutex poisoned")
    }
}
#[cfg(feature = "parking_lot")]
pub trait MutexExt<T> {
    /// Lock the parking_lot mutex (never poisoned).
    fn risky_lock(&self) -> MutexGuard<T>;
}
#[cfg(feature = "parking_lot")]
impl<T> MutexExt<T> for std::sync::Mutex<T> {
    fn risky_lock(&self) -> MutexGuard<T> {
        self.lock()
    }
}

#[derive(Debug)]
pub enum DebounceMode {
    Leading,
    Trailing,
}

struct DebouncerState {
    has_run: bool,
    last_run: Instant,
    triggered: bool,
}

struct DebouncerInner {
    mode: DebounceMode,
    notifier: Notify,
    cooldown: Duration,
    state: Mutex<DebouncerState>,
}

impl DebouncerInner {
    fn finalize(&self, pending: bool) {
        let mut state = self.state.risky_lock();
        if state.triggered {
            state.has_run = true;
            state.triggered = pending;
            state.last_run = tokio::time::Instant::now();
            self.notifier.notify_one();
        }
    }
}

pub struct DebouncerGuard<'a> {
    inner: Arc<DebouncerInner>,
    completed: bool,
    _not_send: PhantomData<*const ()>,
    _not_static: PhantomData<&'a ()>,
}

impl<'a> DebouncerGuard<'a> {
    fn new(inner: Arc<DebouncerInner>) -> Self {
        Self {
            inner,
            completed: false,
            _not_send: PhantomData,
            _not_static: PhantomData,
        }
    }
}

impl<'a> Drop for DebouncerGuard<'a> {
    fn drop(&mut self) {
        if !self.completed {
            let inner = self.inner.clone();
            self.completed = true;
            inner.finalize(false);
        }
    }
}

#[derive(Clone)]
pub struct Debouncer {
    inner: Arc<DebouncerInner>,
}

impl Debouncer {
    pub fn new(cooldown: Duration, mode: DebounceMode) -> Self {
        let inner = Arc::new(DebouncerInner {
            notifier: Notify::new(),
            cooldown,
            state: Mutex::new(DebouncerState {
                has_run: if matches!(mode, DebounceMode::Leading) {
                    false
                } else {
                    true
                },
                last_run: tokio::time::Instant::now(),
                triggered: false,
            }),
            mode,
        });
        Self { inner }
    }

    pub async fn is_triggered(&self) -> bool {
        let state = self.inner.state.risky_lock();
        state.triggered
    }


    pub fn trigger(&self) {
        {
            let mut guard = self.inner.state.risky_lock();
            if matches!(self.inner.mode, DebounceMode::Trailing) {
                guard.last_run = tokio::time::Instant::now();
            }
            if guard.triggered {
                // Already pending, just update the value
                return;
            }
            guard.triggered = true;
        } // guard dropped here
        self.inner.notifier.notify_one();
    }

    pub async fn ready<'a>(&self) -> DebouncerGuard<'a> {
        loop {
            // Phase 1: inspect state (no awaits)
            let action = {
                let state = self.inner.state.risky_lock();

                if !state.triggered {
                    None
                } else {
                    let now = tokio::time::Instant::now();
                    let next_allowed = state.last_run + self.inner.cooldown;

                    match self.inner.mode {
                        DebounceMode::Leading => {
                            if !state.has_run || now >= next_allowed {
                                Some(None)
                            } else {
                                Some(Some(next_allowed))
                            }
                        }
                        DebounceMode::Trailing => {
                            if now >= next_allowed {
                                Some(None)
                            } else {
                                Some(Some(next_allowed))
                            }
                        }
                    }
                }
            }; // ✅ MutexGuard fully dropped here

            // Phase 2: await
            match action {
                None => {
                    self.inner.notifier.notified().await;
                }
                Some(Some(instant)) => {
                    tokio::time::sleep_until(instant).await;
                }
                Some(None) => {
                    break;
                }
            }
        }

        DebouncerGuard::new(self.inner.clone())
    }

}
