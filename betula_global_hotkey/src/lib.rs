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
        let modifiers = mods.unwrap_or_else(Modifiers::empty);
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

struct RemovalHelper {
    fun: Option<Box<dyn FnOnce()>>,
}
impl RemovalHelper {
    pub fn new(fun: Box<dyn FnOnce()>) -> Self {
        Self { fun: Some(fun) }
    }
}

impl Drop for RemovalHelper {
    fn drop(&mut self) {
        if let Some(f) = self.fun.take() {
            (f)()
        }
    }
}

pub struct HotkeyToken {
    state: StatePtr,
    // something that on drop removes the entry
    _remover: RemovalHelper,
}
impl HotkeyToken {
    pub fn is_pressed(&self) -> bool {
        self.state.is_pressed.load(Relaxed)
    }
    pub fn is_toggled(&self) -> bool {
        self.state.is_toggled.load(Relaxed)
    }
}

type TrackedStateMap = Arc<Mutex<HashMap<Hotkey, CountedState>>>;

enum RegistrationTask {
    Register(Hotkey),
    Unregister(Hotkey),
}
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct GlobalHotkeyInterface {
    /// Pointer to the actual manager used by the runner.
    backend: Arc<Mutex<backend::BackendType>>,
    // dead code allowed, it contains the execution thread.
    sender: Sender<RegistrationTask>,

    key_map: TrackedStateMap,

    #[allow(dead_code)]
    _runner: Arc<GlobalHotkeyRunner>,
}

impl GlobalHotkeyInterface {
    pub fn new() -> Result<GlobalHotkeyInterface, BetulaError> {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let t_running = running.clone();

        let key_map: TrackedStateMap = Default::default();

        let backend = Arc::new(Mutex::new(backend::BackendType::new()?));
        let backend_t = Arc::clone(&backend);
        let key_map_t = Arc::clone(&key_map);
        let (sender, receiver) = std::sync::mpsc::channel::<RegistrationTask>();
        let thread = Some(std::thread::spawn(move || {
            let backend = backend_t;
            let key_map = key_map_t;
            while t_running.load(Relaxed) {
                let locked = backend.lock().unwrap();
                while let Ok(v) = receiver.try_recv() {
                    let r = match v {
                        RegistrationTask::Register(key) => locked.register(key),
                        RegistrationTask::Unregister(key) => locked.unregister(key),
                    };
                    if r.is_err() {
                        panic!("error from register or unregister: {:?}", r.err());
                    }
                }
                let events = (*locked).get_events();
                if events.is_err() {
                    return;
                }
                let events = events.unwrap();

                let locked = key_map.lock().unwrap();
                for e in events {
                    if let Some(count_state) = locked.get(&e.hotkey) {
                        let down = e.state == KeyState::Down;
                        count_state.state.is_pressed.store(down, Relaxed);
                        if down {
                            count_state.state.is_toggled.fetch_xor(true, Relaxed);
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }));

        let runner = Arc::new(GlobalHotkeyRunner { thread, running });
        Ok(GlobalHotkeyInterface {
            _runner: runner,
            backend,
            key_map,
            sender,
        })
    }

    pub fn register(&self, key: Hotkey) -> Result<HotkeyToken, HotkeyError> {
        // lock the map
        let (new_registration, state) = {
            let mut locked = self.key_map.lock().unwrap();
            let value = locked.entry(key.clone()).or_default();
            value.count += 1;
            (value.count == 1, Arc::clone(&value.state))
        };

        if new_registration {
            // lets also let the backend know.
            self.sender.send(RegistrationTask::Register(key.clone()))?;
        }

        // Now, crate the removal token
        let removal_fun = {
            let map = Arc::clone(&self.key_map);
            let key_t = key.clone();
            let sender_t = self.sender.clone();
            move || {
                let mut locked = map.lock().unwrap();
                let should_remove = if let Some(v) = locked.get_mut(&key_t) {
                    if v.count == 1 {
                        // It was the last remaining entry!
                        true
                    } else {
                        v.count -= 1;
                        false
                    }
                } else {
                    unreachable!("removal fun called for non existing key");
                };
                if should_remove {
                    // println!("removing {key_t:?} from the map");
                    locked.remove(&key_t);
                    // and lets tell the backend about it.
                    let _ = sender_t.send(RegistrationTask::Unregister(key.clone()));
                }
            }
        };

        let remover = RemovalHelper::new(Box::new(removal_fun));

        Ok(HotkeyToken {
            state,
            _remover: remover,
        })
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
