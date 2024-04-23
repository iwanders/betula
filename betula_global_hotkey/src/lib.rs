/*!
    Betula nodes for global hotkeys.

    Why is this so complicated? Well, because we need to be able to have
    multiple nodes registering different hotkeys and their configuration
    may result in registrations changing. But we need reference counting as
    two nodes may register the same hotkey, and if a node is deleted its
    desire to claim the hotkey needs to be conveyed to the hotkey manager.

    So to do that, registration yields a [`HotkeyToken`], which is both an
    RAII object as well as an interface to the current value of that hotkey.
    When all tokens pointing at the same hotkey go out of scope, the
    registration is cancelled.

    To compound the complexity some more, the Windows side has its own
    implementation that's different from [`global_hotkey`], reason is
    two-fold:

    1. This allows non-blocking keyboard shortcuts on windows, which
       means it could passively detect keystrokes to computer games. I
       had already created this
       [functionality](https://github.com/iwanders/windows_input_hook) so I
       could finally put that to use.
    2. I couldn't get an event loop going in a CLI application, so using
      [`global_hotkey`] was not an option.

*/
use betula_core::BetulaError;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

pub mod nodes;
pub use keyboard_types::{Code, KeyState, Modifiers};

pub type HotkeyError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Description of a particular hotkey.
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

/// Describes a hotkey event from the backend.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
struct HotkeyEvent {
    pub state: KeyState,
    pub hotkey: Hotkey,
}

#[cfg(target_os = "linux")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
mod backend;

#[cfg(target_os = "windows")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod backend;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

/// Struct that holds the state for a particular hotkey.
#[derive(Debug, Default)]
struct State {
    /// Whether the key is currently depressed.
    pub is_pressed: AtomicBool,
    /// Boolean that's toggled when the key is depressed.
    pub is_toggled: AtomicBool,
}

type StatePtr = Arc<State>;

struct GlobalHotkeyRunner {
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

/// Reference counted state.
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

/// Raii object to call a lambda on deletion.
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

/// An interface to a particular hotkey.
pub struct HotkeyToken {
    state: StatePtr,
    hotkey: Hotkey,
    // something that on drop removes the entry
    _remover: RemovalHelper,
}
impl HotkeyToken {
    /// Is the hotkey currently depressed?
    pub fn is_pressed(&self) -> bool {
        self.state.is_pressed.load(Relaxed)
    }
    /// The hotkey toggle state, switches each keydown.
    pub fn is_toggled(&self) -> bool {
        self.state.is_toggled.load(Relaxed)
    }
    /// The hotkey this token is associated to.
    pub fn hotkey(&self) -> &Hotkey {
        &self.hotkey
    }
}

type TrackedStateMap = Arc<Mutex<HashMap<Hotkey, CountedState>>>;

/// Internal enum for communication with the backend management thread.
enum RegistrationTask {
    Register(Hotkey),
    Unregister(Hotkey),
}
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// Interface to the global hotkey system.
#[derive(Clone)]
pub struct GlobalHotkeyInterface {
    // Technically, backend isn't used.
    /// Pointer to the actual manager used by the runner.
    backend: Arc<Mutex<backend::BackendType>>,

    /// Sender to the backend-management thread.
    sender: Sender<RegistrationTask>,

    /// Map of reference counted hotkeys.
    key_map: TrackedStateMap,

    /// Actual runner that manages the backend.
    #[allow(dead_code)]
    _runner: Arc<GlobalHotkeyRunner>,
}

impl GlobalHotkeyInterface {
    /// Create a new instance of the interface and start internal threads.
    pub fn new() -> Result<GlobalHotkeyInterface, BetulaError> {
        let running = std::sync::Arc::new(AtomicBool::new(true));
        let key_map: TrackedStateMap = Default::default();

        let (sender, receiver) = std::sync::mpsc::channel::<RegistrationTask>();
        let backend = Arc::new(Mutex::new(backend::BackendType::new()?));

        // Create the necessary state that's used by the runner.
        let backend_t = Arc::clone(&backend);
        let key_map_t = Arc::clone(&key_map);
        let running_t = running.clone();

        let thread = Some(std::thread::spawn(move || {
            let backend = backend_t;
            let key_map = key_map_t;
            while running_t.load(Relaxed) {
                let locked = backend.lock().unwrap();
                // Handle instructions about registrations.
                while let Ok(v) = receiver.try_recv() {
                    let r = match v {
                        RegistrationTask::Register(key) => locked.register(key),
                        RegistrationTask::Unregister(key) => locked.unregister(key),
                    };
                    if r.is_err() {
                        // don't think this can ever happen?
                        panic!("error from register or unregister: {:?}", r.err());
                    }
                }

                // Obtain the events
                let events = (*locked).get_events();
                if events.is_err() {
                    return;
                }
                let events = events.unwrap();

                // Lock the key map, process all the events and update state atomics.
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

    // Can this function ever return Err?
    /// Register a new hotkey and retrieve the token for it.
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

        // Now, create the removal function.
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
            hotkey: key,
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
