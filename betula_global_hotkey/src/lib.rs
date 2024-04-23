use betula_core::BetulaError;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

pub mod nodes;
pub use keyboard_types::{Code, Modifiers};

pub type HotkeyError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub key: Code,
}
impl Hotkey {
    pub fn new(mods: Option<Modifiers>, key: Code) -> Self {
        let mut modifiers = mods.unwrap_or_else(Modifiers::empty);
        Self { modifiers, key }
    }
}
pub use keyboard_types::KeyState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct HotkeyEvent {
    pub state: KeyState,
    pub hotkey: Hotkey,
}

#[cfg(target_os = "linux")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
mod backend;

#[cfg(target_os = "windows")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod backend;

// From the docs
// On Windows a win32 event loop must be running on the thread. It doesn’t need to be the main thread but you have to create the global hotkey manager on the same thread as the event loop.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
// use std::sync::mpsc::Sender;

#[derive(Debug, Default)]
pub struct State {
    /// Whether the key is currently depressed.
    pub is_pressed: AtomicBool,
    /// Boolean that's toggled when the key is depressed.
    pub is_toggled: AtomicBool,
}

type StatePtr = Arc<State>;

pub struct HotkeyToken {
    state: StatePtr,
    // something that on drop removes the entry
}

pub struct GlobalHotkeyRunner {
    thread: Option<std::thread::JoinHandle<()>>,
    running: std::sync::Arc<AtomicBool>,
}

impl Drop for GlobalHotkeyRunner {
    fn drop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let t = self.thread.take();
        t.unwrap().join().expect("join should succeed");
    }
}

struct CountedState {
    count: usize,
    state: StatePtr,
}
impl Default for CountedState {
    fn default() -> Self {
        Self {
            count: 0,
            state: Default::default(),
        }
    }
}

type TrackedStateMap = Arc<Mutex<HashMap<Hotkey, CountedState>>>;

use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct GlobalHotkeyInterface {
    /// Pointer to the actual manager used by the runner.
    backend: Arc<Mutex<backend::BackendType>>,
    // dead code allowed, it contains the execution thread.
    key_map: TrackedStateMap,

    #[allow(dead_code)]
    runner: Arc<GlobalHotkeyRunner>,
}

impl GlobalHotkeyInterface {
    pub fn new() -> Result<GlobalHotkeyInterface, BetulaError> {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let t_running = running.clone();

        let backend = Arc::new(Mutex::new(backend::BackendType::new()?));
        let backend_t = Arc::clone(&backend);
        let thread = Some(std::thread::spawn(move || {
            let backend = backend_t;
            while t_running.load(Relaxed) {
                let locked = backend.lock().unwrap();
                let events = (*locked).get_events();
                // do something with events... dispatch them to the state map.
            }
        }));

        let runner = Arc::new(GlobalHotkeyRunner { thread, running });
        let key_map = Default::default();
        Ok(GlobalHotkeyInterface {
            runner,
            backend,
            key_map,
        })
    }

    pub fn register(&self, key: Hotkey) -> Result<HotkeyToken, HotkeyError> {
        // lock the map
        let (new_registration, state) = {
            let mut locked = self.key_map.lock().unwrap();
            let mut value = locked.entry(key.clone()).or_default();
            value.count += 1;
            (value.count == 1, Arc::clone(&value.state))
        };

        if new_registration {
            // lets also let the backend know.
            let locked = self.backend.lock().unwrap();
            locked.register(key)?;
        }

        Ok(HotkeyToken { state })
    }
}

impl std::fmt::Debug for GlobalHotkeyInterface {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "GlobalHotkeyRunner<{:?}>", Arc::as_ptr(&self.backend))
    }
}

/// Register global_hotkey nodes to the ui support.
#[cfg(feature = "betula_egui")]
pub fn add_ui_support(ui_support: &mut betula_egui::UiSupport) {}
